use crate::types::ErrorGroup;

/// Build the analysis prompt for an error group
pub fn build_analysis_prompt(group: &ErrorGroup) -> String {
    build_enhanced_analysis_prompt(group, 2000)
}

/// Build enhanced analysis prompt with explicit JSON-only instructions
pub fn build_enhanced_analysis_prompt(group: &ErrorGroup, max_length: usize) -> String {
    // Get example messages (up to 3)
    let examples: Vec<String> = group
        .entries
        .iter()
        .take(3)
        .map(|e| truncate_message(&e.message, max_length))
        .collect();

    let examples_text = if examples.is_empty() {
        truncate_message(&group.pattern, max_length)
    } else {
        examples.join("\n")
    };

    format!(
        r#"You are a Principal Network Engineer and Data Center Architect (L2/L3/EVPN-VXLAN) specializing in Edgecore Networks white-box switches running SONiC (Software for Open Networking in the Cloud) powered by Broadcom Trident ASICs (Trident II / II+ / III / IV). 
Your task is analyze provided error pattern, perform a deep-dive root cause analysis, identify the underlying issue, and provide a clear explanation, troubleshooting and remediation plan in an JSON response.

Input Data:
- Edgecore Switch Models: AS7326-56X / AS5835-54X / AS7726-32X
- SONiC Version: Community SONiC 202311
- Error Pattern: {}
- Occurrences: {}
- Severity: {:?}
- Example Messages: 
{}

Analysis Guidelines (SONiC + Broadcom Trident Context):
When analyzing the logs, strictly account for the SONiC architecture and Broadcom SDK/SAI specifics:
1. Container & Redis DB Interactions: Evaluate the health of critical daemons (`orchagent`, `syncd`, `portsyncd`, `xcvrd`, `teamd`, `bgpd`, `fpmsyncd`, `pmon`). Look for signs of desynchronization between `APPL_DB`, `ASIC_DB`, and `STATE_DB`, as well as SAI API errors/timeouts (`SAI_STATUS_FAILURE`, `SAI_STATUS_TABLE_FULL`, etc.).
2. Hardware Layer (Broadcom Trident & ONLP): Pay close attention to ASIC initialization errors, LPM/EM/FDB/ACL table exhaustion in Trident, I2C/QSFP/SFP+ bus errors (transceiver EEPROM read failures in `xcvrd`), PSU/fan faults (`psud`, `thermalctld`), and Linux kernel/PCIe errors (`dmesg`).
3. Control Plane vs. Data Plane: Clearly distinguish between control-plane issues (e.g., BGP/BFD flaps caused by high CPU load in `bgpd`) and data-plane issues (e.g., physical link drops at the PHY/SerDes level or `syncd` crashes).

Constraints:
- Base your analysis strictly on facts present in the logs. Do not hallucinate unseen errors.
- If the log only shows a symptom (e.g., a BGP neighbor down) without the underlying trigger (e.g., physical link flap or `syncd` restart), explicitly state which additional logs (`/var/log/swss/sairedis.rec`, `/var/log/frr/frr.log`, etc.) are required for a definitive diagnosis.

Required JSON format:
{{
  "explanation": "Clear explanation of what this error means",
  "root_cause": "The underlying cause of this error",
  "suggestions": [
    {{
      "description": "How to fix this issue",
      "code_example": "Optional code example as a string",
      "priority": 1
    }}
  ]
}}
Priority levels: 1 (critical) to 5 (minor)

IMPORTANT: Return ONLY valid JSON. Do not include:
- Markdown code blocks (```json or ```)
- Explanations outside the JSON structure
- Code examples outside the JSON structure
- Any text before or after the JSON object

Return ONLY the JSON object, nothing else."#,
        group.pattern, group.count, group.severity, examples_text
    )
}

/// Truncate message to maximum length
fn truncate_message(message: &str, max_length: usize) -> String {
    if message.len() > max_length {
        format!("{}... (truncated)", &message[..max_length])
    } else {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LogEntry, LogMetadata, Severity};
    use chrono::Utc;

    fn create_test_group() -> ErrorGroup {
        ErrorGroup {
            id: "test-1".to_string(),
            pattern: "NullPointerException".to_string(),
            count: 5,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            severity: Severity::Error,
            entries: vec![LogEntry {
                timestamp: Some(Utc::now()),
                severity: Severity::Error,
                message: "NullPointerException at line 42".to_string(),
                metadata: LogMetadata {
                    file: None,
                    line: None,
                    function: None,
                    thread: None,
                    extra: std::collections::HashMap::new(),
                },
                raw: "".to_string(),
            }],
            analysis: None,
        }
    }

    #[test]
    fn test_build_analysis_prompt() {
        let group = create_test_group();
        let prompt = build_analysis_prompt(&group);

        assert!(prompt.contains("NullPointerException"));
        assert!(prompt.contains("Occurrences: 5"));
        assert!(prompt.contains("ONLY valid JSON"));
        assert!(prompt.contains("Required JSON format"));
    }

    #[test]
    fn test_build_enhanced_analysis_prompt() {
        let group = create_test_group();
        let prompt = build_enhanced_analysis_prompt(&group, 2000);

        assert!(prompt.contains("NullPointerException"));
        assert!(prompt.contains("Return ONLY the JSON object"));
        assert!(prompt.contains("Do not include"));
        assert!(prompt.contains("Markdown code blocks"));
    }

    #[test]
    fn test_truncate_message_short() {
        let message = "Short message";
        let truncated = truncate_message(message, 100);
        assert_eq!(truncated, "Short message");
    }

    #[test]
    fn test_truncate_message_long() {
        let message = "A".repeat(3000);
        let truncated = truncate_message(&message, 2000);
        assert_eq!(truncated.len(), 2015); // 2000 + "... (truncated)"
        assert!(truncated.ends_with("... (truncated)"));
    }

    #[test]
    fn test_prompt_includes_multiple_examples() {
        let mut group = create_test_group();
        group.entries.push(LogEntry {
            timestamp: Some(Utc::now()),
            severity: Severity::Error,
            message: "NullPointerException at line 43".to_string(),
            metadata: LogMetadata {
                file: None,
                line: None,
                function: None,
                thread: None,
                extra: std::collections::HashMap::new(),
            },
            raw: "".to_string(),
        });

        let prompt = build_enhanced_analysis_prompt(&group, 2000);
        assert!(prompt.contains("line 42"));
        assert!(prompt.contains("line 43"));
    }

    #[test]
    fn test_prompt_limits_to_three_examples() {
        let mut group = create_test_group();
        for i in 1..=5 {
            group.entries.push(LogEntry {
                timestamp: Some(Utc::now()),
                severity: Severity::Error,
                message: format!("Error {}", i),
                metadata: LogMetadata {
                    file: None,
                    line: None,
                    function: None,
                    thread: None,
                    extra: std::collections::HashMap::new(),
                },
                raw: "".to_string(),
            });
        }

        let prompt = build_enhanced_analysis_prompt(&group, 2000);
        // Should only include first 3 examples from entries
        assert!(prompt.contains("line 42")); // Original entry
        assert!(prompt.contains("Error 1"));
        assert!(prompt.contains("Error 2"));
        assert!(!prompt.contains("Error 3")); // 4th entry, should be excluded
        assert!(!prompt.contains("Error 4"));
        assert!(!prompt.contains("Error 5"));
    }
}
