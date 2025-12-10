import subprocess
import time
import requests
import pytest
import os
from typing import Optional


class ServerProcess:
    """Manages the lifecycle of the ping-viewer-next server process."""

    def __init__(self, server_address: str = "0.0.0.0:8080", binary_path: Optional[str] = None, attach_only: bool = False):
        self.server_address = server_address
        self.binary_path = binary_path
        self.attach_only = attach_only
        self.process: Optional[subprocess.Popen] = None
        self.base_url = f"http://{server_address}"

    def start(self, timeout: int = 30) -> None:
        env = os.environ.copy()
        env.setdefault("RUST_LOG", "warn")

        if self.attach_only:
            print(f"Attaching to external server at {self.base_url} (no process will be spawned)")
        elif self.binary_path:
            print(f"Using provided binary: {self.binary_path}")
            abs_binary_path = os.path.abspath(self.binary_path)
            if not os.path.exists(abs_binary_path):
                raise RuntimeError(f"Provided binary path does not exist: {abs_binary_path}")
            binary_dir = os.path.dirname(abs_binary_path)

            print("Starting the ping-viewer-next server...")
            self.process = subprocess.Popen(
                [abs_binary_path, "--rest-server", self.server_address],
                cwd=binary_dir,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env=env
            )
        else:
            print("Building the Rust project...")
            project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

            build_result = subprocess.run(
                ["cargo", "build"],
                cwd=project_root,
                capture_output=True,
                text=True,
                timeout=60
            )

            if build_result.returncode != 0:
                raise RuntimeError(f"Failed to build project: {build_result.stderr}")

            print("Starting the ping-viewer-next server...")
            self.process = subprocess.Popen(
                ["./target/debug/ping-viewer-next", "--rest-server", self.server_address],
                cwd=project_root,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env=env
            )

        self._wait_for_server(timeout)

    def _wait_for_server(self, timeout: int) -> None:
        start_time = time.time()

        while time.time() - start_time < timeout:
            try:
                response = requests.get(f"{self.base_url}/register_service", timeout=5)
                if response.status_code == 200:
                    print(f"Server is ready at {self.base_url}")
                    return
            except requests.RequestException:
                pass

            time.sleep(0.5)

        self.stop()
        raise TimeoutError(f"Server did not start within {timeout} seconds")

    def stop(self) -> None:
        if self.process:
            print("Stopping the server...")
            try:
                self.process.terminate()
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                print("Force killing server process...")
                self.process.kill()
                self.process.wait()
            finally:
                self.process = None

    def is_running(self) -> bool:
        return self.process is not None and self.process.poll() is None


class TestServerIntegration:
    """Test class that reuses the same server instance across multiple tests."""

    server = None

    @classmethod
    def setup_class(cls):
        """Set up the server once for all tests in this class."""
        print("Setting up server for test class...")

        binary_path = getattr(pytest, "PVN_BINARY_PATH", None)
        server_address = getattr(pytest, "PVN_SERVER_ADDRESS", None)
        if binary_path:
            print(f"Using custom binary path from CLI: {binary_path}")

        if server_address:
            print(f"Attaching to provided server at: {server_address}")
            cls.server = ServerProcess(server_address=server_address, binary_path=None, attach_only=True)
        else:
            cls.server = ServerProcess(binary_path=binary_path)
        cls.server.start()
        cls.working_endpoint = cls._find_working_endpoint()

        print("Waiting for device discovery before running tests...")
        wait_seconds = 30
        print(f"Waiting {wait_seconds} seconds for devices to be discovered...")

        import time
        time.sleep(wait_seconds)

        print("Device discovery wait completed. Checking for discovered devices...")

        response = requests.get(f"{cls.server.base_url}{cls.working_endpoint}")
        if response.status_code == 200:
            devices_data = response.json()
            if "DeviceInfo" in devices_data and devices_data["DeviceInfo"]:
                device_count = len(devices_data["DeviceInfo"])
                print(f"✓ Found {device_count} device(s) after discovery wait")
            else:
                print("ℹ️  No devices found after discovery wait (this is normal)")
        else:
            print(f"⚠️  Could not list devices after discovery wait: {response.status_code}")

        print("Server setup completed - tests can now run with discovered devices")

    @classmethod
    def teardown_class(cls):
        """Clean up the server after all tests in this class."""
        print("Cleaning up server after test class...")
        if cls.server:
            cls.server.stop()

    @classmethod
    def _find_working_endpoint(cls):
        """Find the working device list endpoint."""
        endpoints_to_try = [
            "/device_manager/List",
            "/v1/device_manager/List",
        ]

        for endpoint in endpoints_to_try:
            try:
                response = requests.get(f"{cls.server.base_url}{endpoint}")
                print(f"Testing endpoint {endpoint}: status {response.status_code}")
                if response.status_code in [200, 500]:
                    return endpoint
            except Exception as e:
                print(f"Endpoint {endpoint} error: {e}")
                continue

        raise RuntimeError("Could not find working device list endpoint")

    def _list_devices(self):
        response = requests.get(f"{self.server.base_url}{self.working_endpoint}")
        return response

    def _get_ping1d_revision(self, device):
        props = device.get("properties") or {}
        p1d = props.get("Ping1D") or {}
        common = p1d.get("common") or {}
        info = common.get("device_information") or {}
        return info.get("device_revision")

    def _monitor_firmware_update_status(self, device_uuid, device_type):
        """Monitor firmware update status for a device using the List API.

        Args:
            device_uuid: UUID of the device being updated
            device_type: Type of device (Ping1D or Ping360)
        """
        print(f"Monitoring firmware update status for {device_type} device {device_uuid}...")

        import time
        max_wait_time = 5 * 60
        check_interval = 5
        start_time = time.time()

        print(f"Monitoring firmware update progress for up to {max_wait_time // 60} minutes...")

        while time.time() - start_time < max_wait_time:
            list_response = self._list_devices()
            if list_response.status_code != 200:
                print(f"⚠️  Failed to list devices: {list_response.status_code}")
                time.sleep(check_interval)
                continue

            list_data = list_response.json()
            if "DeviceInfo" not in list_data:
                print("⚠️  Unexpected device list response format")
                time.sleep(check_interval)
                continue

            target_device = None
            for device in list_data["DeviceInfo"]:
                if device.get("id") == device_uuid:
                    target_device = device
                    break

            if not target_device:
                print(f"⚠️  Target device {device_uuid} not found in device list")
                time.sleep(check_interval)
                continue

            current_status = target_device.get("status", "Unknown")
            elapsed_time = int(time.time() - start_time)

            if current_status == "Available":
                print(f"[{elapsed_time}s] Device status: {current_status}")
                print(f"✓ Firmware update completed successfully after {elapsed_time} seconds")
                return
            elif current_status == "Bootloader":
                print(f"[{elapsed_time}s] Device status: {current_status}")
                print(f"❌ Firmware update failed - device entered bootloader mode after {elapsed_time} seconds")
                pytest.fail(f"Firmware update failed for {device_type} {device_uuid} - device stuck in bootloader mode")
            elif current_status != "Updating":
                print(f"⚠️  Unexpected status change to {current_status} after {elapsed_time} seconds")
            time.sleep(check_interval)
        print(f"❌ Firmware update timed out after {max_wait_time} seconds")
        pytest.fail(f"Firmware update timed out for {device_type} {device_uuid} after {max_wait_time // 60} minutes")

    def test_list_current_devices(self):
        print("Running first device list test...")

        response = self._list_devices()

        assert response.status_code in [200, 500], f"Expected 200 or 500, got {response.status_code}"

        if response.status_code == 200:
            # Parse JSON response
            data = response.json()
            print(f"Device list response: {data}")
            # Basic validation - ensure we got some kind of response
            assert isinstance(data, dict), "Response should be a JSON object"
        else:
            # Check that it's the expected NoDevices error
            assert "NoDevices" in response.text, f"Expected NoDevices error, got: {response.text}"
            print("Received expected NoDevices response - no devices are currently connected")

    def test_list_devices_after_discovery_wait(self):
        """After discovery wait, require presence of specific devices and sources:
        - Ping1D over Serial (rev 1)
        - Ping2 over Serial (rev 2, reported under Ping1D with revision=2)
        - Ping360 over Serial
        - Ping360 over UDP
        """
        print("=== Testing Device List After Discovery Wait ===")
        print("Note: The 2-minute discovery wait already completed during server setup")

        response = self._list_devices()

        # Require success and a device list
        assert response.status_code == 200, f"Failed to list devices after discovery wait: {response.status_code} - {response.text}"
        data = response.json()
        assert isinstance(data, dict), "Response should be a JSON object"
        assert "DeviceInfo" in data, "Missing DeviceInfo in response"
        devices = data["DeviceInfo"] or []
        device_count = len(devices)
        print(f"✓ Found {device_count} device(s) available for testing")

        found_ping1d_serial = False
        found_ping2_serial = False
        found_ping360_serial = False
        found_ping360_udp = False

        def get_revision_from_properties(d):
            props = d.get("properties") or {}
            p1d = props.get("Ping1D") or {}
            common = p1d.get("common") or {}
            info = common.get("device_information") or {}
            return info.get("device_revision")

        for i, device in enumerate(devices):
            device_id = device.get("id", "unknown")
            device_type = device.get("device_type", "unknown")
            source = device.get("source", {})
            source_type = list(source.keys())[0] if source else "unknown"
            print(f"  Device {i+1}: {device_id} ({device_type} via {source_type})")

            if device_type == "Ping1D" and "SerialStream" in source:
                revision = get_revision_from_properties(device)
                if revision == 2:
                    found_ping2_serial = True
                else:
                    # Treat non-2 revisions as Ping1D
                    found_ping1d_serial = True

            if device_type == "Ping360":
                if "SerialStream" in source:
                    found_ping360_serial = True
                if "UdpStream" in source:
                    found_ping360_udp = True

        missing = []
        if not found_ping1d_serial:
            missing.append("Ping1D over Serial")
        if not found_ping2_serial:
            missing.append("Ping2 over Serial")
        if not found_ping360_serial:
            missing.append("Ping360 over Serial")
        if not found_ping360_udp:
            missing.append("Ping360 over UDP")

        if missing:
            pytest.fail("Missing required devices after discovery: " + ", ".join(missing))

    def test_device_operations_sequence(self):
        """Test device operations: check existence, enable continuous mode, wait, disable, delete."""
        target_uuid = "00000000-0000-0000-ca48-b4211fd08699"
        print(f"=== Testing device operations for UUID: {target_uuid} ===")

        # Step 1: Check if device exists by listing all devices
        print("Step 1: Listing devices to check if target device exists...")
        response = self._list_devices()

        if response.status_code == 500 and "NoDevices" in response.text:
            print(f"❌ Device {target_uuid} does not exist (no devices connected)")
            pytest.skip(f"Device {target_uuid} not found - no devices are currently connected")

        assert response.status_code == 200, f"Failed to list devices: {response.status_code}"
        devices_data = response.json()

        # Check if the target device exists
        device_exists = False
        if "DeviceInfo" in devices_data:
            devices = devices_data["DeviceInfo"]
            for device in devices:
                if device.get("id") == target_uuid:
                    device_exists = True
                    print(f"✓ Found target device: {device}")
                    break

        if not device_exists:
            print(f"❌ Device {target_uuid} does not exist in the current device list")
            pytest.skip(f"Device {target_uuid} not found in device list")

        # Step 2: Enable continuous mode
        print("Step 2: Enabling continuous mode...")
        continuous_url = f"{self.server.base_url}/device_manager/{target_uuid}/EnableContinuousMode"
        response = requests.post(continuous_url)

        if response.status_code == 200:
            print("✓ Successfully enabled continuous mode")
        else:
            print(f"⚠️  Failed to enable continuous mode: {response.status_code} - {response.text}")
            # Continue with the test even if this fails, as it might be expected

        # Step 3: Wait 1 minute
        print("Step 3: Waiting 1 minute with continuous mode enabled...")
        time.sleep(60)  # 1 minute = 60 seconds
        print("✓ Waited 1 minute")

        # Step 4: Disable continuous mode
        print("Step 4: Disabling continuous mode...")
        disable_url = f"{self.server.base_url}/device_manager/{target_uuid}/DisableContinuousMode"
        response = requests.post(disable_url)

        if response.status_code == 200:
            print("✓ Successfully disabled continuous mode")
        time.sleep(60)  # 1 minute = 60 seconds to wait for the device to be disabled
        print("✓ Waited 1 minute")

        # Step 5: Delete the device
        print("Step 5: Deleting the device...")
        delete_url = f"{self.server.base_url}/device_manager/{target_uuid}/Delete"
        response = requests.post(delete_url)

        if response.status_code == 200:
            print("✓ Successfully deleted the device")
        else:
            print(f"❌ Failed to delete device: {response.status_code} - {response.text}")
            # This is more critical, so we'll assert it worked
            assert response.status_code == 200, f"Failed to delete device: {response.text}"

        print(f"✓ Completed all operations for device {target_uuid}")

    def test_ping1d_firmware_update(self):
        """Test firmware update for Ping1D (revision 1) devices connected via serial.

        NOTE: This test should run after test_wait_5_minutes_and_list_devices
        to allow time for devices to be discovered.
        """
        print("=== Testing Ping1D (rev1) Serial Firmware Updates (after device discovery) ===")

        # Get list of devices
        response = self._list_devices()

        if response.status_code == 500 and "NoDevices" in response.text:
            print("❌ No devices connected - skipping Ping1D firmware update test")
            pytest.skip("No devices connected")

        assert response.status_code == 200, f"Failed to list devices: {response.status_code}"
        devices_data = response.json()

        # Find Ping1D rev1 devices with serial source
        ping1d_serial_devices = []
        if "DeviceInfo" in devices_data:
            for device in devices_data["DeviceInfo"]:
                device_type = device.get("device_type")
                source = device.get("source", {})

                # Check if it's a Ping1D device with serial source and revision == 1
                if device_type == "Ping1D" and "SerialStream" in source and self._get_ping1d_revision(device) == 1:
                    ping1d_serial_devices.append(device)
                    print(f"✓ Found Ping1D device with serial source: {device['id']}")

        if not ping1d_serial_devices:
            print("❌ No Ping1D (rev1) devices with serial source found")
            pytest.skip("No Ping1D (rev1) devices with serial source found")

        # Update firmware for each Ping1D device found
        for device in ping1d_serial_devices:
            device_uuid = device["id"]
            print(f"Updating firmware for Ping1D device: {device_uuid}")

            # Prepare firmware update request
            firmware_request = {
                "mode": {
                    "AutoUpdate": {
                        "uuid": device_uuid
                    }
                },
                "force": False
            }

            # Send firmware update request
            update_response = requests.post(
                f"{self.server.base_url}/device_manager/firmware_update",
                json=firmware_request
            )

            if update_response.status_code == 200:
                result = update_response.json()
                print(f"✓ Firmware update initiated for {device_uuid}: {result}")

                # Monitor firmware update status
                self._monitor_firmware_update_status(device_uuid, "Ping1D")
            else:
                print(f"⚠️  Firmware update failed for {device_uuid}: {update_response.status_code} - {update_response.text}")

        print(f"✓ Completed firmware update checks for {len(ping1d_serial_devices)} Ping1D (rev1) serial devices")

    def test_ping2_firmware_update(self):
        """Test firmware update for Ping2 (revision 2) devices connected via serial."""
        print("=== Testing Ping2 (rev2) Serial Firmware Updates (after device discovery) ===")

        # Get list of devices
        response = self._list_devices()

        if response.status_code == 500 and "NoDevices" in response.text:
            print("❌ No devices connected - skipping Ping2 firmware update test")
            pytest.skip("No devices connected")

        assert response.status_code == 200, f"Failed to list devices: {response.status_code}"
        devices_data = response.json()

        # Find Ping2 devices (Ping1D with revision 2) with serial source
        ping2_serial_devices = []
        if "DeviceInfo" in devices_data:
            for device in devices_data["DeviceInfo"]:
                device_type = device.get("device_type")
                source = device.get("source", {})
                if device_type == "Ping1D" and "SerialStream" in source and self._get_ping1d_revision(device) == 2:
                    ping2_serial_devices.append(device)
                    print(f"✓ Found Ping2 (rev2) device with serial source: {device['id']}")

        if not ping2_serial_devices:
            print("❌ No Ping2 (rev2) devices with serial source found")
            pytest.skip("No Ping2 (rev2) devices with serial source found")

        for device in ping2_serial_devices:
            device_uuid = device["id"]
            print(f"Updating firmware for Ping2 device: {device_uuid}")

            firmware_request = {
                "mode": {
                    "AutoUpdate": {
                        "uuid": device_uuid
                    }
                },
                "force": False
            }

            update_response = requests.post(
                f"{self.server.base_url}/device_manager/firmware_update",
                json=firmware_request
            )

            if update_response.status_code == 200:
                result = update_response.json()
                print(f"✓ Firmware update initiated for {device_uuid}: {result}")
                self._monitor_firmware_update_status(device_uuid, "Ping2")
            else:
                print(f"⚠️  Firmware update failed for {device_uuid}: {update_response.status_code} - {update_response.text}")

        print(f"✓ Completed firmware update checks for {len(ping2_serial_devices)} Ping2 (rev2) serial devices")

    def test_ping360_firmware_update(self):
        """Test firmware update for Ping360 devices connected via serial.

        NOTE: This test should run after test_wait_5_minutes_and_list_devices
        to allow time for devices to be discovered.
        """
        print("=== Testing Ping360 Serial Firmware Updates (after device discovery) ===")

        # Get list of devices
        response = self._list_devices()

        if response.status_code == 500 and "NoDevices" in response.text:
            print("❌ No devices connected - skipping Ping360 firmware update test")
            pytest.skip("No devices connected")

        assert response.status_code == 200, f"Failed to list devices: {response.status_code}"
        devices_data = response.json()

        # Find Ping360 devices with serial source
        ping360_serial_devices = []
        if "DeviceInfo" in devices_data:
            for device in devices_data["DeviceInfo"]:
                device_type = device.get("device_type")
                source = device.get("source", {})

                # Check if it's a Ping360 device with serial source
                if device_type == "Ping360" and "SerialStream" in source:
                    ping360_serial_devices.append(device)
                    print(f"✓ Found Ping360 device with serial source: {device['id']}")

        if not ping360_serial_devices:
            print("❌ No Ping360 devices with serial source found")
            pytest.skip("No Ping360 devices with serial source found")

        # Update firmware for each Ping360 device found
        for device in ping360_serial_devices:
            device_uuid = device["id"]
            print(f"Updating firmware for Ping360 device: {device_uuid}")

            # Prepare firmware update request
            firmware_request = {
                "mode": {
                    "AutoUpdate": {
                        "uuid": device_uuid
                    }
                },
                "force": False
            }

            # Send firmware update request
            update_response = requests.post(
                f"{self.server.base_url}/device_manager/firmware_update",
                json=firmware_request
            )

            if update_response.status_code == 200:
                result = update_response.json()
                print(f"✓ Firmware update initiated for {device_uuid}: {result}")

                # Monitor firmware update status
                self._monitor_firmware_update_status(device_uuid, "Ping360")
            else:
                print(f"⚠️  Firmware update failed for {device_uuid}: {update_response.status_code} - {update_response.text}")

        print(f"✓ Completed firmware update checks for {len(ping360_serial_devices)} Ping360 serial devices")

    def test_server_health_check(self):
        """Test that the server is still healthy and responsive."""
        print("Checking server health...")

        # Test the register service endpoint
        response = requests.get(f"{self.server.base_url}/register_service")
        assert response.status_code == 200, f"Server health check failed: {response.status_code}"

        data = response.json()
        assert "name" in data, "Server metadata should contain name"
        assert data["name"] == "Ping Viewer Next", f"Expected 'Ping Viewer Next', got {data['name']}"

        print("Server is healthy and responsive")

    def test_concurrent_firmware_updates_all_kinds(self):
        """Trigger Ping1D, Ping2, and Ping360 (all serial) firmware updates concurrently and monitor them in parallel."""
        print("=== Testing concurrent firmware updates for Ping1D, Ping2, and Ping360 (serial) ===")

        # List devices
        response = self._list_devices()
        if response.status_code == 500 and "NoDevices" in response.text:
            print("❌ No devices connected - skipping concurrent firmware update test")
            pytest.skip("No devices connected")
        assert response.status_code == 200, f"Failed to list devices: {response.status_code}"
        devices_data = response.json()
        devices = devices_data.get("DeviceInfo", []) if isinstance(devices_data, dict) else []

        # Pick one device of each kind (serial)
        ping1d_rev1 = None
        ping2_rev2 = None
        ping360_serial = None
        for device in devices:
            device_type = device.get("device_type")
            source = device.get("source", {})
            if device_type == "Ping1D" and "SerialStream" in source:
                revision = self._get_ping1d_revision(device)
                if revision == 1 and not ping1d_rev1:
                    ping1d_rev1 = device
                elif revision == 2 and not ping2_rev2:
                    ping2_rev2 = device
            elif device_type == "Ping360" and "SerialStream" in source and not ping360_serial:
                ping360_serial = device

        missing = []
        if not ping1d_rev1:
            missing.append("Ping1D (rev1) serial")
        if not ping2_rev2:
            missing.append("Ping2 (rev2) serial")
        if not ping360_serial:
            missing.append("Ping360 serial")
        if missing:
            pytest.skip("Missing required devices for concurrent update: " + ", ".join(missing))

        print(f"Selected devices for concurrent update:")
        print(f"  - Ping1D (rev1): {ping1d_rev1['id']}")
        print(f"  - Ping2 (rev2): {ping2_rev2['id']}")
        print(f"  - Ping360: {ping360_serial['id']}")

        # Prepare requests
        targets = [
            ("Ping1D", ping1d_rev1["id"]),
            ("Ping2", ping2_rev2["id"]),
            ("Ping360", ping360_serial["id"]),
        ]

        def send_update(uuid):
            req = {
                "mode": {
                    "AutoUpdate": {
                        "uuid": uuid
                    }
                },
                "force": False
            }
            return requests.post(f"{self.server.base_url}/device_manager/firmware_update", json=req)

        # Fire updates concurrently
        from concurrent.futures import ThreadPoolExecutor, as_completed
        print("Starting concurrent firmware update requests...")
        with ThreadPoolExecutor(max_workers=6) as executor:
            future_to_target = {executor.submit(send_update, uuid): (dtype, uuid) for dtype, uuid in targets}
            for future in as_completed(future_to_target):
                dtype, uuid = future_to_target[future]
                try:
                    res = future.result()
                except Exception as e:
                    pytest.fail(f"Failed to start firmware update for {dtype} {uuid}: {e}")
                else:
                    if res.status_code == 200:
                        print(f"✓ Update started for {dtype} {uuid}: {res.json()}")
                    else:
                        pytest.fail(f"Failed to start firmware update for {dtype} {uuid}: {res.status_code} - {res.text}")

        # Monitor all three in parallel
        print("Monitoring all firmware updates in parallel...")
        with ThreadPoolExecutor(max_workers=6) as executor:
            monitor_futures = [executor.submit(self._monitor_firmware_update_status, uuid, dtype) for dtype, uuid in targets]
            for future in as_completed(monitor_futures):
                # Exceptions inside monitor will raise and fail the test with pytest.fail
                _ = future.result()

        print("✓ Concurrent firmware updates for all kinds completed successfully")
