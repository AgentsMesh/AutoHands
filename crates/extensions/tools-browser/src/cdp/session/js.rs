//! JavaScript execution operations for CDP page session.

use serde_json::{json, Value};

use crate::cdp::error::CdpError;
use crate::cdp::protocol::RemoteObject;

use super::core::PageSession;

impl PageSession {
    /// Evaluate JavaScript expression.
    pub async fn evaluate(&self, expression: &str) -> Result<Value, CdpError> {
        let result = self
            .call(
                "Runtime.evaluate",
                Some(json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                })),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception["text"].as_str().unwrap_or("Unknown error");
            return Err(CdpError::JavaScript(text.to_string()));
        }

        Ok(result["result"]["value"].clone())
    }

    /// Evaluate JavaScript and return remote object.
    pub async fn evaluate_handle(&self, expression: &str) -> Result<RemoteObject, CdpError> {
        let result = self
            .call(
                "Runtime.evaluate",
                Some(json!({
                    "expression": expression,
                    "returnByValue": false,
                })),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception["text"].as_str().unwrap_or("Unknown error");
            return Err(CdpError::JavaScript(text.to_string()));
        }

        let remote_obj: RemoteObject = serde_json::from_value(result["result"].clone())?;
        Ok(remote_obj)
    }

    /// Call function on remote object.
    pub async fn call_function_on(
        &self,
        object_id: &str,
        function: &str,
        args: Option<Vec<Value>>,
    ) -> Result<Value, CdpError> {
        let mut params = json!({
            "objectId": object_id,
            "functionDeclaration": function,
            "returnByValue": true,
            "awaitPromise": true,
        });

        if let Some(a) = args {
            params["arguments"] = json!(a.into_iter().map(|v| json!({"value": v})).collect::<Vec<_>>());
        }

        let result = self.call("Runtime.callFunctionOn", Some(params)).await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception["text"].as_str().unwrap_or("Unknown error");
            return Err(CdpError::JavaScript(text.to_string()));
        }

        Ok(result["result"]["value"].clone())
    }
}
