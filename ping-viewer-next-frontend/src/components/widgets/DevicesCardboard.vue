<template>
  <div class="bg-gray-100 min-h-screen p-6">
    <div class="max-w-7xl mx-auto bg-white shadow-lg rounded-lg overflow-hidden">
      <div class="bg-gray-800 text-white py-4 px-6">
        <h2 class="text-2xl font-bold">Running Devices</h2>
      </div>

      <div class="p-6">
        <div v-if="devices.length === 0" class="text-gray-500 text-center py-8">
          No devices found.
        </div>
        <div v-else class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
          <div
            v-for="device in devices"
            :key="device.id"
            class="bg-gray-800 text-white p-5 rounded-lg shadow-md"
          >
            <h3 class="text-lg font-semibold mb-3">{{ device.device_type }}</h3>
            <p class="mb-2"><span class="font-medium">ID:</span> {{ device.id }}</p>
            <p class="mb-2"><span class="font-medium">Status:</span> {{ device.status }}</p>
            <div v-if="device.source.SerialStream" class="mt-4">
              <p class="mb-2"><span class="font-medium">Serial Path:</span> {{ device.source.SerialStream.path }}</p>
              <p><span class="font-medium">Baudrate:</span> {{ device.source.SerialStream.baudrate }}</p>
            </div>
            <div v-if="device.source.UdpStream" class="mt-4">
              <p class="mb-2"><span class="font-medium">IP:</span> {{ device.source.UdpStream.ip }}</p>
              <p><span class="font-medium">Port:</span> {{ device.source.UdpStream.port }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  data() {
    return {
      devices: [],
    };
  },
  mounted() {
    this.fetchDevices();
  },
  methods: {
    async fetchDevices() {
      try {
        const response = await fetch(`${window.location.protocol}//${window.location.host}/device_manager/List`, {
          headers: {
            accept: "application/json",
          },
        });
        const data = await response.json();
        this.devices = data.DeviceInfo;
      } catch (error) {
        console.error("Error fetching devices:", error);
      }
    },
  },
};
</script>
