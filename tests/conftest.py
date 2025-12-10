import pytest
import os

BINARY_PATH = None


def pytest_addoption(parser):
    """Register custom CLI options for controlling how the server binary is provided."""
    group = parser.getgroup("ping-viewer")
    group.addoption(
        "--binary",
        action="store",
        default=None,
        help="Use an existing ping-viewer-next binary (provide absolute path).",
    )
    group.addoption(
        "--build",
        action="store_true",
        default=False,
        help="Build the project before running tests (default behavior if --binary is not provided).",
    )
    group.addoption(
        "--server",
        action="store",
        default=None,
        help="Attach to an already running server at IP:PORT (e.g., 127.0.0.1:8080).",
    )


@pytest.fixture(scope="session", autouse=True)
def _configure_binary_or_build(request):
    """Configure environment for tests based on CLI args.

    Requirements:
    - Exactly one of:
      * --binary PATH (use existing binary)
      * --build (build before running)
      * --server IP:PORT (attach to an already running server)
    """
    global BINARY_PATH
    BINARY_PATH = None

    binary_opt = request.config.getoption("--binary")
    build_opt = request.config.getoption("--build")
    server_opt = request.config.getoption("--server")

    if sum(bool(x) for x in [binary_opt, build_opt, server_opt]) != 1:
        raise pytest.UsageError(
            "[conftest] You must specify exactly one of: --binary PATH, --build, or --server IP:PORT"
        )

    if binary_opt:
        # Explicit path: ensure it exists
        abs_path = os.path.abspath(binary_opt)
        if not os.path.exists(abs_path):
            raise pytest.UsageError(
                f"[conftest] --binary path does not exist: {abs_path}"
            )
        BINARY_PATH = abs_path
        print(f"[conftest] Using provided binary: {abs_path}")
        # No external server in this mode
        setattr(pytest, "PVN_SERVER_ADDRESS", None)
    elif server_opt:
        # Attach to external server; do not manage a binary
        BINARY_PATH = None
        setattr(pytest, "PVN_SERVER_ADDRESS", server_opt)
        print(f"[conftest] Attaching to external server at: {server_opt}")
    else:
        # Explicit build mode; expose None to tests
        BINARY_PATH = None
        setattr(pytest, "PVN_SERVER_ADDRESS", None)

    # Expose the resolved path to tests without using env vars
    setattr(pytest, "PVN_BINARY_PATH", BINARY_PATH)


