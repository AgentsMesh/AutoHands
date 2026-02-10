//! DOM operations for CDP page session.

use serde_json::json;

use crate::cdp::error::CdpError;
use crate::cdp::protocol::{AXNode, BoxModel, ComputedStyle, DomNode, EventListener, RemoteObject};

use super::core::PageSession;

impl PageSession {
    /// Get document root node.
    pub async fn get_document(&self) -> Result<DomNode, CdpError> {
        let result = self
            .call(
                "DOM.getDocument",
                Some(json!({"depth": -1, "pierce": true})),
            )
            .await?;

        let root: DomNode = serde_json::from_value(result["root"].clone())?;
        Ok(root)
    }

    /// Query selector.
    pub async fn query_selector(&self, selector: &str) -> Result<Option<i64>, CdpError> {
        let doc = self.get_document().await?;

        let result = self
            .call(
                "DOM.querySelector",
                Some(json!({
                    "nodeId": doc.node_id,
                    "selector": selector,
                })),
            )
            .await?;

        let node_id = result["nodeId"].as_i64().unwrap_or(0);
        if node_id == 0 {
            Ok(None)
        } else {
            Ok(Some(node_id))
        }
    }

    /// Query selector all.
    pub async fn query_selector_all(&self, selector: &str) -> Result<Vec<i64>, CdpError> {
        let doc = self.get_document().await?;

        let result = self
            .call(
                "DOM.querySelectorAll",
                Some(json!({
                    "nodeId": doc.node_id,
                    "selector": selector,
                })),
            )
            .await?;

        let node_ids: Vec<i64> = result["nodeIds"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();

        Ok(node_ids)
    }

    /// Get box model for node.
    pub async fn get_box_model(&self, node_id: i64) -> Result<Option<BoxModel>, CdpError> {
        let result = self
            .call("DOM.getBoxModel", Some(json!({"nodeId": node_id})))
            .await;

        match result {
            Ok(r) => {
                let model: BoxModel = serde_json::from_value(r["model"].clone())?;
                Ok(Some(model))
            }
            Err(CdpError::Protocol { code: -32000, .. }) => {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Get computed style for node.
    pub async fn get_computed_style(&self, node_id: i64) -> Result<Vec<ComputedStyle>, CdpError> {
        let result = self
            .call(
                "CSS.getComputedStyleForNode",
                Some(json!({"nodeId": node_id})),
            )
            .await?;

        let styles: Vec<ComputedStyle> =
            serde_json::from_value(result["computedStyle"].clone()).unwrap_or_default();

        Ok(styles)
    }

    /// Get event listeners for node.
    pub async fn get_event_listeners(
        &self,
        object_id: &str,
    ) -> Result<Vec<EventListener>, CdpError> {
        let result = self
            .call(
                "DOMDebugger.getEventListeners",
                Some(json!({"objectId": object_id})),
            )
            .await?;

        let listeners: Vec<EventListener> =
            serde_json::from_value(result["listeners"].clone()).unwrap_or_default();

        Ok(listeners)
    }

    /// Resolve node to runtime object.
    pub async fn resolve_node(&self, node_id: i64) -> Result<RemoteObject, CdpError> {
        let result = self
            .call("DOM.resolveNode", Some(json!({"nodeId": node_id})))
            .await?;

        let obj: RemoteObject = serde_json::from_value(result["object"].clone())?;
        Ok(obj)
    }

    /// Focus element.
    pub async fn focus(&self, node_id: i64) -> Result<(), CdpError> {
        self.call("DOM.focus", Some(json!({"nodeId": node_id})))
            .await?;
        Ok(())
    }

    /// Set node value (for input elements).
    pub async fn set_node_value(&self, node_id: i64, value: &str) -> Result<(), CdpError> {
        self.focus(node_id).await?;
        self.press_key_combo("Control+a").await?;
        self.type_text(value).await?;
        Ok(())
    }

    /// Click on element by selector.
    pub async fn click_selector(&self, selector: &str) -> Result<(), CdpError> {
        let node_id = self
            .query_selector(selector)
            .await?
            .ok_or_else(|| CdpError::ElementNotFound(selector.to_string()))?;

        let box_model = self
            .get_box_model(node_id)
            .await?
            .ok_or_else(|| CdpError::ElementNotFound(format!("{} (not visible)", selector)))?;

        let (x, y) = Self::quad_center(&box_model.content);
        self.click(x, y).await
    }

    /// Fill input by selector.
    pub async fn fill(&self, selector: &str, value: &str) -> Result<(), CdpError> {
        let node_id = self
            .query_selector(selector)
            .await?
            .ok_or_else(|| CdpError::ElementNotFound(selector.to_string()))?;

        self.set_node_value(node_id, value).await
    }

    /// Calculate center point of a quad.
    pub(super) fn quad_center(quad: &[f64]) -> (f64, f64) {
        if quad.len() >= 8 {
            let x = (quad[0] + quad[2] + quad[4] + quad[6]) / 4.0;
            let y = (quad[1] + quad[3] + quad[5] + quad[7]) / 4.0;
            (x, y)
        } else {
            (0.0, 0.0)
        }
    }

    /// Get accessibility tree.
    pub async fn get_accessibility_tree(&self) -> Result<Vec<AXNode>, CdpError> {
        self.call("Accessibility.enable", None).await?;
        let result = self.call("Accessibility.getFullAXTree", None).await?;
        let nodes: Vec<AXNode> =
            serde_json::from_value(result["nodes"].clone()).unwrap_or_default();
        Ok(nodes)
    }
}
