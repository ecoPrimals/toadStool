# WebSocket Tests Removed (Feb 2026)

WebSocket support was removed from ToadStool. Real-time events now use JSON-RPC 2.0 polling via biomeOS/songbird coordination over Unix sockets.

Deleted test files (previously covered websocket.rs):
- websocket_comprehensive_tests.rs
- websocket_comprehensive_coverage.rs
- websocket_unit_tests.rs
- websocket_tests.rs
- websocket_expansion_tests.rs
- websocket_logic_tests.rs
- websocket_month1_tests.rs
- websocket_real_tests.rs

ServerEvent::to_json() tests are in state_and_events_tests.rs.
