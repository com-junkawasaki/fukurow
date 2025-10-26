//! JSON-LD serialization and deserialization utilities

use crate::model::{JsonLdDocument, Triple, CyberEvent};
use serde_json;
use anyhow::{Result, anyhow};

/// Convert JSON-LD document to triples
pub fn jsonld_to_triples(doc: &JsonLdDocument) -> Result<Vec<Triple>> {
    let mut triples = Vec::new();

    if let Some(graph) = &doc.graph {
        for node in graph {
            if let Some(node_obj) = node.as_object() {
                if let Some(subject) = node_obj.get("@id") {
                    let subject_str = subject.as_str()
                        .ok_or_else(|| anyhow!("@id must be a string"))?;

                    for (key, value) in node_obj {
                        if key != "@id" && key != "@type" {
                            if let Some(obj_str) = value.as_str() {
                                triples.push(Triple {
                                    subject: subject_str.to_string(),
                                    predicate: key.clone(),
                                    object: obj_str.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(triples)
}

/// Convert cyber event to JSON-LD
pub fn cyber_event_to_jsonld(event: &CyberEvent) -> Result<JsonLdDocument> {
    let (event_type, data) = match event {
        CyberEvent::NetworkConnection { source_ip, dest_ip, port, protocol, timestamp } => {
            ("NetworkConnection", serde_json::json!({
                "sourceIp": source_ip,
                "destIp": dest_ip,
                "port": port,
                "protocol": protocol,
                "timestamp": timestamp
            }))
        },
        CyberEvent::ProcessExecution { process_id, parent_process_id, command_line, user, timestamp } => {
            ("ProcessExecution", serde_json::json!({
                "processId": process_id,
                "parentProcessId": parent_process_id,
                "commandLine": command_line,
                "user": user,
                "timestamp": timestamp
            }))
        },
        CyberEvent::FileAccess { file_path, access_type, user, process_id, timestamp } => {
            ("FileAccess", serde_json::json!({
                "filePath": file_path,
                "accessType": access_type,
                "user": user,
                "processId": process_id,
                "timestamp": timestamp
            }))
        },
        CyberEvent::UserLogin { user, source_ip, success, timestamp } => {
            ("UserLogin", serde_json::json!({
                "user": user,
                "sourceIp": source_ip,
                "success": success,
                "timestamp": timestamp
            }))
        },
    };

    let context = serde_json::json!({
        "@vocab": "https://w3id.org/security#",
        "sourceIp": "https://w3id.org/security#sourceIp",
        "destIp": "https://w3id.org/security#destIp",
        "port": "https://w3id.org/security#port",
        "protocol": "https://w3id.org/security#protocol",
        "timestamp": "https://w3id.org/security#timestamp",
        "processId": "https://w3id.org/security#processId",
        "parentProcessId": "https://w3id.org/security#parentProcessId",
        "commandLine": "https://w3id.org/security#commandLine",
        "user": "https://w3id.org/security#user",
        "filePath": "https://w3id.org/security#filePath",
        "accessType": "https://w3id.org/security#accessType",
        "success": "https://w3id.org/security#success"
    });

    let node = serde_json::json!({
        "@type": event_type,
        "@id": format!("_:event_{}", uuid::Uuid::new_v4()),
        "@graph": [data]
    });

    Ok(JsonLdDocument {
        context,
        graph: Some(vec![node]),
        data: std::collections::HashMap::new(),
    })
}

/// Parse JSON-LD string to document
pub fn parse_jsonld(json_str: &str) -> Result<JsonLdDocument> {
    let doc: JsonLdDocument = serde_json::from_str(json_str)?;
    Ok(doc)
}

/// Serialize JSON-LD document to string
pub fn serialize_jsonld(doc: &JsonLdDocument) -> Result<String> {
    let json_str = serde_json::to_string_pretty(doc)?;
    Ok(json_str)
}
