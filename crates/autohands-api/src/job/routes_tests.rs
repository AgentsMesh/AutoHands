//! Tests for job routes.

use super::*;
use crate::job::definition::{Job, JobDefinition};

#[test]
fn test_job_list_response_serialization() {
    let response = JobListResponse {
        count: 0,
        jobs: vec![],
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["count"], 0);
    assert!(json["jobs"].as_array().unwrap().is_empty());
}

#[test]
fn test_job_response_serialization() {
    let def = JobDefinition::new("test-job", "0 * * * *", "test-agent", "Do task");
    let job = Job::new(def);
    let response = JobResponse { job };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["job"]["definition"]["id"], "test-job");
    assert_eq!(json["job"]["definition"]["schedule"], "0 * * * *");
}
