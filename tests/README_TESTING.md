# Testing Ping Viewer Next

## Running the Server Tests

The project includes pytest tests that start the Rust server and test its API endpoints.

### Prerequisites

1. Python 3.12+ with pip
2. Rust toolchain (cargo) - or a pre-built ping-viewer-next binary

### Setup

1. Create a virtual environment:

```bash
python3 -m venv venv
source venv/bin/activate
```

2. Install Python dependencies:

```bash
pip install -r tests/requirements.txt
```

### Running Tests

Run all server integration tests from project root:

```bash
pytest tests/test_server.py --binary ./target/release/ping-viewer-next -v
```

The test suite will:

1. Start the ping-viewer-next server on `0.0.0.0:8080` once for all tests
2. **Wait 2 minutes for device discovery during setup** (this ensures all tests have access to discovered devices)
3. Run multiple tests against the same server instance
4. Stop the server after all tests complete
