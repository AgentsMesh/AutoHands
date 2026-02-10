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
    tool_registry: Arc<ToolRegistry>,
    config: AgentLoopConfig,
}

impl StreamingAgentLoop {
    pub fn new(
        _provider_registry: Arc<ProviderRegistry>,
        tool_registry: Arc<ToolRegistry>,
        config: AgentLoopConfig,
    ) -> Self {
        Self {
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

        let tool_registry = self.tool_registry.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let executor = StreamExecutor {
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
#[path = "streaming_tests.rs"]
mod tests;
