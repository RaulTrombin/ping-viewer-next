<template>
  <div class="min-h-screen w-full flex flex-col bg-black">
    <div class="flex-none bg-gray-800 p-4 flex justify-between items-center">
      <div class="flex items-center">
        <h1 class="text-2xl font-bold text-white">Multi-Device Viewer</h1>
        <v-chip v-if="selectedDevices.length > 0" color="primary" class="ml-4">
          {{ selectedDevices.length }} Device{{ selectedDevices.length !== 1 ? 's' : '' }} Selected
        </v-chip>
      </div>

      <div class="flex items-center">
        <v-btn color="primary" @click="showDeviceDialog = true">
          <v-icon icon="mdi-plus" class="mr-2" />
          Add Device
        </v-btn>
      </div>
    </div>

    <v-main class="flex-grow h-full w-full bg-black overflow-y-auto">
      <div v-if="selectedDevices.length === 0" class="h-full flex items-center justify-center">
        <v-btn color="primary" size="large" @click="showDeviceDialog = true">
          Add Devices to View
        </v-btn>
      </div>

      <div v-else class="w-full p-4 grid gap-4 auto-rows-min grid-cols-2 grid-rows-2">
        <div v-for="device in selectedDevices" :key="device.id"
          class="relative bg-gray-900 rounded-lg overflow-hidden min-h-[300px]">
          <div class="absolute top-0 right-0 z-10 p-2 flex gap-2">
            <v-btn icon="mdi-close" size="small" color="error" variant="text" @click="removeDevice(device)" />
          </div>

          <component :is="getDeviceComponent(device)" :device="device" :websocketUrl="getWebSocketUrl(device)"
            v-bind="getDeviceProps(device)" class="h-full w-full" />
        </div>
      </div>
    </v-main>

    <v-dialog v-model="showDeviceDialog" max-width="800px">
      <v-card>
        <v-card-title class="text-h5 bg-gray-800 text-white">
          Select Devices
        </v-card-title>

        <v-card-text class="pa-6">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <DeviceCard v-for="device in availableDevices" :key="device.id" :device="device"
              :selected="isDeviceSelected(device)" :showActions="true" @toggle="toggleDevice(device)" />
          </div>
        </v-card-text>

        <v-card-actions class="pa-4">
          <v-spacer />
          <v-btn color="primary" @click="showDeviceDialog = false">
            Done
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<script setup>
import { useDeviceFetching } from '@/composables/useDeviceFetching';
import { computed, inject, ref } from 'vue';
import DeviceCard from '../utils/DeviceCard.vue';
import Ping1DLoader from '../widgets/sonar1d/Ping1DLoader.vue';
import Ping360Loader from '../widgets/sonar360/Ping360Loader.vue';

const props = defineProps({
  serverUrl: {
    type: String,
    required: true,
  },
});

const { commonSettings, ping1DSettings, ping360Settings } = inject('deviceSettings');

const { deviceInfo } = useDeviceFetching(props.serverUrl);

const availableDevices = computed(() => {
  return deviceInfo.value.DeviceInfo?.filter((device) => device.status === 'ContinuousMode') || [];
});

const selectedDevices = ref([]);
const showDeviceDialog = ref(false);

const toggleDevice = (device) => {
  const index = selectedDevices.value.findIndex((d) => d.id === device.id);
  if (index === -1) {
    selectedDevices.value.push(device);
  } else {
    selectedDevices.value.splice(index, 1);
  }
};

const removeDevice = (device) => {
  const index = selectedDevices.value.findIndex((d) => d.id === device.id);
  if (index !== -1) {
    selectedDevices.value.splice(index, 1);
  }
};

const isDeviceSelected = (device) => {
  return selectedDevices.value.some((d) => d.id === device.id);
};

const getDeviceComponent = (device) => {
  return device.device_type === 'Ping360' ? Ping360Loader : Ping1DLoader;
};

const getDeviceProps = (device) => {
  const settings = device.device_type === 'Ping360' ? ping360Settings : ping1DSettings;

  const width = Math.floor((window.innerWidth - 48) / 2); // 48 = 3 * 16 (gap)
  const height = Math.floor((window.innerHeight - 200 - 48) / 2); // 48 = 3 * 16 (gap)

  return {
    ...commonSettings,
    ...settings,
    width,
    height,
  };
};

const getWebSocketUrl = (device) => {
  if (!device) return '';
  const protocol = props.serverUrl.startsWith('https') ? 'wss:' : 'ws:';
  const host = props.serverUrl.replace(/^https?:\/\//, '');
  return `${protocol}//${host}/ws?device_number=${device.id}`;
};
</script>

<style scoped>
.device-card {
  transition: all 0.3s ease;
}

.device-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
}
</style>