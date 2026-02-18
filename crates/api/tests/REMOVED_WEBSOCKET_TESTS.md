# WebSocket Tests Removed (Feb 2026)

WebSocket support was removed from ToadStool API. Real-time events now use JSON-RPC 2.0 polling via biomeOS/songbird coordination over Unix sockets.

Deleted test files:
- websocket_comprehensive_tests.rs
- websocket_test.rs
- websocket_integration.rs
