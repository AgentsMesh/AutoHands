//! Streaming response support for agent loop.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::mpsc;
use tracing::debug;

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_protocols::agent::{Agent, AgentContext};
use autohands_protocols::error::AgentError;
use autohands_protocols::provider::{ChunkType, CompletionChunk};
use autohands_protocols::tool::ToolContext;
use autohands_protocols::types::{Message, ToolCall};

use crate::AgentLoopConfig;

/// Event emitted during streaming execution.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Start of a new turn.
    TurnStart { turn: u32 },
    /// Text content delta.
    TextDelta { content: String },
    /// Tool call started.
    ToolCallStart { id: String, name: String },
    /// Tool call input delta.
    ToolCallDelta { id: String, input_delta: String },
    /// Tool call completed.
    ToolCallComplete { id: String, result: String },
    /// Turn completed.
    TurnComplete { turn: u32 },
    /// Agent completed.
    Complete { message: Message },
    /// Error occurred.
    Error { error: String },
}

/// Stream of agent events.
pub struct AgentEventStream {
    receiver: mpsc::Receiver<StreamEvent>,
}

impl Stream for AgentEventStream {
    type Item = StreamEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_recv(cx)
    }
}

/// Streaming agent loop executor.
pub struct StreamingAgentLoop {
    provider_registry: Arc<ProviderRegistry>,
    tool_registry: Arc<ToolRegistry>,
    config: AgentLoopConfig,
}

impl StreamingAgentLoop {
    pub fn new(
        provider_registry: Arc<ProviderRegistry>,
        tool_registry: Arc<ToolRegistry>,
        config: AgentLoopConfig,
    ) -> Self {
        Self {
            provider_registry,
            tool_registry,
            config,
        }
    }

    /// Run the streaming agent loop.
    pub fn run_stream(
        &self,
        agent: Arc<dyn Agent>,
        ctx: AgentContext,
        initial_message: Message,
    ) -> AgentEventStream {
        let (tx, rx) = mpsc::channel(100);

        let provider_registry = self.provider_registry.clone();
        let tool_registry = self.tool_registry.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let executor = StreamExecutor {
                provider_registry,
                tool_registry,
                config,
                tx,
            };
            let _ = executor.execute(agent, ctx, initial_message).await;
        });

        AgentEventStream { receiver: rx }
    }
}

struct StreamExecutor {
    #[allow(dead_code)]
    provider_registry: Arc<ProviderRegistry>,
    tool_registry: Arc<ToolRegistry>,
    config: AgentLoopConfig,
    tx: mpsc::Sender<StreamEvent>,
}

impl StreamExecutor {
    async fn execute(
        &self,
        agent: Arc<dyn Agent>,
        mut ctx: AgentContext,
        initial_message: Message,
    ) -> Result<(), AgentError> {
        let mut messages = ctx.history.clone();
        messages.push(initial_message);

        let mut turn = 0;

        loop {
            if ctx.abort_signal.is_aborted() {
                self.send(StreamEvent::Error {
                    error: "Aborted".to_string(),
                })
                .await;
                return Err(AgentError::Aborted);
            }

            if turn >= self.config.max_turns {
                self.send(StreamEvent::Error {
                    error: format!("Max turns exceeded: {}", turn),
                })
                .await;
                return Err(AgentError::MaxTurnsExceeded(turn));
            }

            turn += 1;
            self.send(StreamEvent::TurnStart { turn }).await;
            debug!("Streaming agent loop turn {}", turn);

            // Process through agent
            ctx.history = messages.clone();
            let response = match agent
                .process(messages.last().unwrap().clone(), ctx.clone())
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    self.send(StreamEvent::Error {
                        error: e.to_string(),
                    })
                    .await;
                    return Err(e);
                }
            };

            messages.push(response.message.clone());

            // Emit text content
            let text = response.message.content.text();
            if !text.is_empty() {
                self.send(StreamEvent::TextDelta { content: text }).await;
            }

            if response.is_complete {
                self.send(StreamEvent::TurnComplete { turn }).await;
                self.send(StreamEvent::Complete {
                    message: response.message,
                })
                .await;
                break;
            }

            // Handle tool calls
            for tool_call in &response.tool_calls {
                self.send(StreamEvent::ToolCallStart {
                    id: tool_call.id.clone(),
                    name: tool_call.name.clone(),
                })
                .await;

                let result = self.execute_tool(tool_call, &ctx).await;

                self.send(StreamEvent::ToolCallComplete {
                    id: tool_call.id.clone(),
                    result: result.clone(),
                })
                .await;

                let tool_message = Message::tool(&tool_call.id, result);
                messages.push(tool_message);
            }

            self.send(StreamEvent::TurnComplete { turn }).await;
        }

        Ok(())
    }

    async fn send(&self, event: StreamEvent) {
        let _ = self.tx.send(event).await;
    }

    async fn execute_tool(&self, tool_call: &ToolCall, ctx: &AgentContext) -> String {
        let tool = match self.tool_registry.get(&tool_call.name) {
            Some(t) => t,
            None => return format!("Tool not found: {}", tool_call.name),
        };

        let tool_ctx = ToolContext::new(&ctx.session_id, std::env::current_dir().unwrap());

        match tool.execute(tool_call.arguments.clone(), tool_ctx).await {
            Ok(result) => result.content,
            Err(e) => format!("Tool error: {}", e),
        }
    }
}

/// Process completion chunks into stream events.
pub struct ChunkProcessor {
    current_text: String,
    current_tool_id: Option<String>,
    current_tool_name: Option<String>,
    current_tool_input: String,
}

impl ChunkProcessor {
    pub fn new() -> Self {
        Self {
            current_text: String::new(),
            current_tool_id: None,
            current_tool_name: None,
            current_tool_input: String::new(),
        }
    }

    /// Process a completion chunk and return events.
    pub fn process(&mut self, chunk: &CompletionChunk) -> Vec<StreamEvent> {
        let mut events = Vec::new();

        match chunk.chunk_type {
            ChunkType::ContentDelta => {
                if let Some(ref delta) = chunk.delta {
                    self.current_text.push_str(delta);
                    events.push(StreamEvent::TextDelta {
                        content: delta.clone(),
                    });
                }
            }
            ChunkType::ToolUseStart => {
                if let Some(ref tc) = chunk.tool_call {
                    self.current_tool_id = tc.id.clone();
                    self.current_tool_name = tc.name.clone();
                    self.current_tool_input.clear();

                    if let (Some(id), Some(name)) = (&self.current_tool_id, &self.current_tool_name)
                    {
                        events.push(StreamEvent::ToolCallStart {
                            id: id.clone(),
                            name: name.clone(),
                        });
                    }
                }
            }
            ChunkType::ToolUseDelta => {
                if let Some(ref tc) = chunk.tool_call {
                    if let Some(ref input) = tc.input_delta {
                        self.current_tool_input.push_str(input);

                        if let Some(ref id) = self.current_tool_id {
                            events.push(StreamEvent::ToolCallDelta {
                                id: id.clone(),
                                input_delta: input.clone(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        events
    }

    /// Get accumulated text.
    pub fn text(&self) -> &str {
        &self.current_text
    }

    /// Reset the processor.
    pub fn reset(&mut self) {
        self.current_text.clear();
        self.current_tool_id = None;
        self.current_tool_name = None;
        self.current_tool_input.clear();
    }
}

impl Default for ChunkProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_event_variants() {
        let event = StreamEvent::TurnStart { turn: 1 };
        if let StreamEvent::TurnStart { turn } = event {
            assert_eq!(turn, 1);
        }

        let event = StreamEvent::TextDelta {
            content: "hello".to_string(),
        };
        if let StreamEvent::TextDelta { content } = event {
            assert_eq!(content, "hello");
        }
    }

    #[test]
    fn test_chunk_processor_new() {
        let processor = ChunkProcessor::new();
        assert!(processor.text().is_empty());
    }

    #[test]
    fn test_chunk_processor_default() {
        let processor = ChunkProcessor::default();
        assert!(processor.text().is_empty());
    }

    #[test]
    fn test_chunk_processor_text_delta() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: Some("Hello".to_string()),
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert_eq!(events.len(), 1);

        if let StreamEvent::TextDelta { content } = &events[0] {
            assert_eq!(content, "Hello");
        } else {
            panic!("Expected TextDelta event");
        }

        assert_eq!(processor.text(), "Hello");
    }

    #[test]
    fn test_chunk_processor_accumulates_text() {
        let mut processor = ChunkProcessor::new();

        let chunks = vec![
            CompletionChunk {
                chunk_type: ChunkType::ContentDelta,
                delta: Some("Hello".to_string()),
                tool_call: None,
                stop_reason: None,
                usage: None,
            },
            CompletionChunk {
                chunk_type: ChunkType::ContentDelta,
                delta: Some(" World".to_string()),
                tool_call: None,
                stop_reason: None,
                usage: None,
            },
        ];

        for chunk in chunks {
            processor.process(&chunk);
        }

        assert_eq!(processor.text(), "Hello World");
    }

    #[test]
    fn test_chunk_processor_reset() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: Some("Hello".to_string()),
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        processor.process(&chunk);
        assert_eq!(processor.text(), "Hello");

        processor.reset();
        assert!(processor.text().is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        let start_chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("test_tool".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&start_chunk);
        assert_eq!(events.len(), 1);

        if let StreamEvent::ToolCallStart { id, name } = &events[0] {
            assert_eq!(id, "call_1");
            assert_eq!(name, "test_tool");
        } else {
            panic!("Expected ToolCallStart event");
        }
    }

    #[test]
    fn test_streaming_agent_loop_creation() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let config = AgentLoopConfig::default();

        let _loop = StreamingAgentLoop::new(provider_registry, tool_registry, config);
    }

    #[test]
    fn test_stream_event_tool_call_start() {
        let event = StreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "test".to_string(),
        };
        if let StreamEvent::ToolCallStart { id, name } = event {
            assert_eq!(id, "call_1");
            assert_eq!(name, "test");
        }
    }

    #[test]
    fn test_stream_event_tool_call_delta() {
        let event = StreamEvent::ToolCallDelta {
            id: "call_1".to_string(),
            input_delta: "{\"key\":".to_string(),
        };
        if let StreamEvent::ToolCallDelta { id, input_delta } = event {
            assert_eq!(id, "call_1");
            assert_eq!(input_delta, "{\"key\":");
        }
    }

    #[test]
    fn test_stream_event_tool_call_complete() {
        let event = StreamEvent::ToolCallComplete {
            id: "call_1".to_string(),
            result: "success".to_string(),
        };
        if let StreamEvent::ToolCallComplete { id, result } = event {
            assert_eq!(id, "call_1");
            assert_eq!(result, "success");
        }
    }

    #[test]
    fn test_stream_event_turn_complete() {
        let event = StreamEvent::TurnComplete { turn: 5 };
        if let StreamEvent::TurnComplete { turn } = event {
            assert_eq!(turn, 5);
        }
    }

    #[test]
    fn test_stream_event_complete() {
        let msg = Message::assistant("Done");
        let event = StreamEvent::Complete { message: msg };
        if let StreamEvent::Complete { message } = event {
            assert_eq!(message.content.text(), "Done");
        }
    }

    #[test]
    fn test_stream_event_error() {
        let event = StreamEvent::Error {
            error: "Something went wrong".to_string(),
        };
        if let StreamEvent::Error { error } = event {
            assert_eq!(error, "Something went wrong");
        }
    }

    #[test]
    fn test_chunk_processor_tool_use_delta() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // First start a tool use
        let start_chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("test_tool".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        processor.process(&start_chunk);

        // Then add delta
        let delta_chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: Some("{\"arg\":".to_string()),
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&delta_chunk);
        assert_eq!(events.len(), 1);

        if let StreamEvent::ToolCallDelta { id, input_delta } = &events[0] {
            assert_eq!(id, "call_1");
            assert_eq!(input_delta, "{\"arg\":");
        } else {
            panic!("Expected ToolCallDelta event");
        }
    }

    #[test]
    fn test_chunk_processor_other_chunk_types() {
        let mut processor = ChunkProcessor::new();

        // MessageStart should produce no events
        let chunk = CompletionChunk {
            chunk_type: ChunkType::MessageStart,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());

        // MessageDelta without delta should produce no events
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_event_debug() {
        let event = StreamEvent::TurnStart { turn: 1 };
        let debug = format!("{:?}", event);
        assert!(debug.contains("TurnStart"));

        let event = StreamEvent::Error { error: "test".to_string() };
        let debug = format!("{:?}", event);
        assert!(debug.contains("Error"));
    }

    #[test]
    fn test_stream_event_clone() {
        let event = StreamEvent::TextDelta { content: "hello".to_string() };
        let cloned = event.clone();
        if let StreamEvent::TextDelta { content } = cloned {
            assert_eq!(content, "hello");
        }
    }

    #[test]
    fn test_chunk_processor_tool_use_start_without_id() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // Tool use start without id should not produce event
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: Some("test".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_start_without_name() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: None,
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_delta_without_prior_start() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // Delta without prior start should produce no event (no current_tool_id)
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: Some("{\"key\":".to_string()),
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_delta_without_input() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // First start a tool
        let start_chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("test".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        processor.process(&start_chunk);

        // Delta without input_delta
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_start_without_tool_call() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_delta_without_tool_call() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_reset_clears_tool_state() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // Start a tool
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("test".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        processor.process(&chunk);

        // Add some input
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: Some("{\"key\":".to_string()),
            }),
            stop_reason: None,
            usage: None,
        };
        processor.process(&chunk);

        // Reset and verify tool state is cleared
        processor.reset();

        // Now delta should produce no events since there's no active tool
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: Some("value}".to_string()),
            }),
            stop_reason: None,
            usage: None,
        };
        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_message_end() {
        use autohands_protocols::types::StopReason;

        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::MessageEnd,
            delta: None,
            tool_call: None,
            stop_reason: Some(StopReason::EndTurn),
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_message_start() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::MessageStart,
            delta: Some("ignored".to_string()),
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        // MessageStart doesn't produce TextDelta events
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_event_all_variants_clone() {
        let events = vec![
            StreamEvent::TurnStart { turn: 1 },
            StreamEvent::TextDelta { content: "hi".to_string() },
            StreamEvent::ToolCallStart { id: "1".to_string(), name: "t".to_string() },
            StreamEvent::ToolCallDelta { id: "1".to_string(), input_delta: "x".to_string() },
            StreamEvent::ToolCallComplete { id: "1".to_string(), result: "r".to_string() },
            StreamEvent::TurnComplete { turn: 1 },
            StreamEvent::Complete { message: Message::assistant("done") },
            StreamEvent::Error { error: "err".to_string() },
        ];

        for event in events {
            let _cloned = event.clone();
        }
    }

    #[test]
    fn test_streaming_agent_loop_with_custom_config() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let config = AgentLoopConfig {
            max_turns: 10,
            timeout_seconds: 60,
            checkpoint_enabled: false,
        };

        let _loop = StreamingAgentLoop::new(provider_registry, tool_registry, config);
    }

    #[test]
    fn test_chunk_processor_multiple_tool_uses() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // First tool
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("tool1".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        let events = processor.process(&chunk);
        assert_eq!(events.len(), 1);

        // Second tool (overwrites first)
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_2".to_string()),
                name: Some("tool2".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        let events = processor.process(&chunk);
        assert_eq!(events.len(), 1);

        if let StreamEvent::ToolCallStart { id, name } = &events[0] {
            assert_eq!(id, "call_2");
            assert_eq!(name, "tool2");
        }
    }
}
