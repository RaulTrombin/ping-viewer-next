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


@pytest.fixture(scope="session", autouse=True)
def _configure_binary_or_build(request):
    """Configure environment for tests based on CLI args.

    Requirements:
    - Exactly one of:
      * --binary PATH (use existing binary)
      * --build (build before running)
    """
    global BINARY_PATH
    BINARY_PATH = None

    binary_opt = request.config.getoption("--binary")
    build_opt = request.config.getoption("--build")

    if (binary_opt and build_opt) or (not binary_opt and not build_opt):
        raise pytest.UsageError(
            "[conftest] You must specify exactly one of: --binary PATH or --build"
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
    else:
        # Explicit build mode; expose None to tests
        BINARY_PATH = None

    # Expose the resolved path to tests without using env vars
    setattr(pytest, "PVN_BINARY_PATH", BINARY_PATH)


