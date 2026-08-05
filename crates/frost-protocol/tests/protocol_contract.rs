use frost_protocol::{
    EventEnvelope, HostCommandEnvelope, HostCommandResultEnvelope, ProtocolRequest,
    ProtocolResponse,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ContractFixture {
    version: u16,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Scenario {
    name: String,
    request: ProtocolRequest,
    response: ProtocolResponse,
    event: EventEnvelope,
    host_command: HostCommandEnvelope,
    host_result: HostCommandResultEnvelope,
}

#[derive(Debug, Deserialize)]
struct NegativeFixture {
    cases: Vec<NegativeCase>,
}

#[derive(Debug, Deserialize)]
struct NegativeCase {
    name: String,
    payload: String,
    expected: String,
}

fn contract_fixture() -> ContractFixture {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/protocol/contract.json"
    ))
    .expect("shared protocol fixture must be valid JSON and Frost Protocol")
}

#[test]
fn shared_fixture_covers_ten_boundary_scenarios() {
    let fixture = contract_fixture();

    assert_eq!(fixture.version, 0);
    assert_eq!(fixture.scenarios.len(), 10);
    assert!(fixture.scenarios.iter().all(|scenario| {
        scenario.request.version == 0
            && scenario.response.version == 0
            && scenario.event.version == 0
            && scenario.host_command.version == 0
            && scenario.host_result.version == 0
    }));

    let names: Vec<&str> = fixture
        .scenarios
        .iter()
        .map(|scenario| scenario.name.as_str())
        .collect();
    for required in [
        "tabs-create",
        "tabs-close",
        "tabs-move",
        "navigation-open",
        "window-create",
        "rollback-failure",
    ] {
        assert!(names.contains(&required), "missing scenario {required}");
    }
}

#[test]
fn shared_fixture_round_trips_without_field_drift() {
    let fixture = contract_fixture();

    for scenario in fixture.scenarios {
        let request: ProtocolRequest = serde_json::from_value(
            serde_json::to_value(&scenario.request).expect("request serializes"),
        )
        .expect("request round-trips");
        let response: ProtocolResponse = serde_json::from_value(
            serde_json::to_value(&scenario.response).expect("response serializes"),
        )
        .expect("response round-trips");
        let event: EventEnvelope = serde_json::from_value(
            serde_json::to_value(&scenario.event).expect("event serializes"),
        )
        .expect("event round-trips");
        let command: HostCommandEnvelope = serde_json::from_value(
            serde_json::to_value(&scenario.host_command).expect("command serializes"),
        )
        .expect("command round-trips");
        let result: HostCommandResultEnvelope = serde_json::from_value(
            serde_json::to_value(&scenario.host_result).expect("result serializes"),
        )
        .expect("result round-trips");

        assert_eq!(request.version, 0);
        assert_eq!(response.version, 0);
        assert_eq!(event.version, 0);
        assert_eq!(command.version, 0);
        assert_eq!(result.version, 0);
    }
}

#[test]
fn negative_fixture_rejects_malformed_unknown_version_and_missing_fields() {
    let fixture: NegativeFixture = serde_json::from_str(include_str!(
        "../../../tests/fixtures/protocol/negative.json"
    ))
    .expect("negative fixture must be valid JSON");
    assert_eq!(fixture.cases.len(), 4);
    assert!(fixture.cases.iter().all(|case| case.expected == "invalid"));

    for case in fixture.cases {
        let value = serde_json::from_str::<serde_json::Value>(&case.payload);
        match case.name.as_str() {
            "malformed-json" => assert!(value.is_err()),
            "unknown-version" => {
                let request: ProtocolRequest = serde_json::from_value(value.unwrap()).unwrap();
                assert_ne!(
                    request.version, 0,
                    "unknown version must be rejected by callers"
                );
            }
            "missing-field" => {
                assert!(serde_json::from_str::<EventEnvelope>(&case.payload).is_err());
            }
            "wrong-type" => {
                assert!(serde_json::from_str::<HostCommandResultEnvelope>(&case.payload).is_err());
            }
            name => panic!("unexpected negative fixture {name}"),
        }
    }
}
