<template>
  <div class="h-full w-full bg-transparent" ref="containerRef">
    <!-- Loading State -->
    <div v-if="isLoading" class="h-full w-full flex items-center justify-center text-white">
      <div class="text-center">
        <v-progress-circular indeterminate color="primary" size="64" class="mb-4" />
        <div>Connecting to device...</div>
      </div>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="h-full w-full flex items-center justify-center">
      <div class="text-center p-4 max-w-md text-white">
        <v-icon color="error" size="48" class="mb-4">mdi-alert-circle</v-icon>
        <h2 class="text-xl mb-2">Error Loading Widget</h2>
        <p class="text-gray-400">{{ error }}</p>
        <div class="mt-4 text-left text-sm bg-gray-800 p-4 rounded">
          <div><strong>Type:</strong> {{ route.params.type }}</div>
          <div><strong>Server URL:</strong> {{ serverUrl }}</div>
          <div><strong>Device ID:</strong> {{ deviceId }}</div>
          <div><strong>WebSocket URL:</strong> {{ websocketUrl }}</div>
        </div>
      </div>
    </div>

    <!-- Widget Display -->
    <component v-else-if="widgetComponent && deviceData" :is="widgetComponent" v-bind="widgetProps"
      class="h-full w-full bg-transparent" />
  </div>
</template>
<script setup>
import Ping1DLoader from '@/components/widgets/sonar1d/Ping1DLoader.vue';
import Ping360Loader from '@/components/widgets/sonar360/Ping360Loader.vue';
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';
import { useRoute } from 'vue-router';

const route = useRoute();
const containerRef = ref(null);
const serverUrl = ref('');
const deviceId = ref('');
const error = ref('');
const isLoading = ref(true);
const deviceData = ref(null);
const dimensions = ref({ width: 0, height: 0 });

// Add resize observer
let resizeObserver = null;

// Update dimensions on resize
const updateDimensions = () => {
  if (!containerRef.value) return;
  const rect = containerRef.value.getBoundingClientRect();
  dimensions.value = {
    width: rect.width,
    height: rect.height,
  };
};

// Determine which widget to load based on route param
const widgetType = computed(() => route.params.type?.toLowerCase());

// Get component based on type
const widgetComponent = computed(() => {
  switch (widgetType.value) {
    case 'ping360':
      return Ping360Loader;
    case 'ping1d':
      return Ping1DLoader;
    default:
      return null;
  }
});

// Compute websocket URL
const websocketUrl = computed(() => {
  if (!serverUrl.value || !deviceId.value) return '';
  const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = new URL(serverUrl.value).host;
  return `${wsProtocol}//${host}/ws?device_number=${deviceId.value}`;
});

// Default widget props
const commonProps = {
  width: window.innerWidth,
  height: window.innerHeight,
  colorPalette: 'Ocean',
};

const ping360Props = {
  ...commonProps,
  lineColor: '#f44336',
  lineWidth: 0.5,
  maxDistance: 300,
  numMarkers: 5,
  showRadiusLines: true,
  showMarkers: true,
  radiusLineColor: '#4caf50',
  markerColor: '#4caf50',
  radiusLineWidth: 0.5,
};

const ping1DProps = {
  ...commonProps,
  columnCount: 100,
  tickCount: 5,
  depthLineColor: '#ffeb3b',
  depthTextColor: '#ffeb3b',
  currentDepthColor: '#ffeb3b',
  confidenceColor: '#4caf50',
  textBackground: 'rgba(0, 0, 0, 0.5)',
  depthArrowColor: '#f44336',
};

// Get props based on widget type
const widgetProps = computed(() => {
  if (!deviceData.value) return {};

  const baseProps = {
    device: deviceData.value,
    websocketUrl: websocketUrl.value,
    width: dimensions.value.width,
    height: dimensions.value.height,
    showControls: false,
  };

  if (widgetType.value === 'ping360') {
    return {
      ...baseProps,
      ...ping360Props,
      width: dimensions.value.width,
      height: dimensions.value.height,
    };
  }
  return {
    ...baseProps,
    ...ping1DProps,
    width: dimensions.value.width,
    height: dimensions.value.height,
    columnCount: Math.floor(dimensions.value.width / 20), // Adjust column count based on width
  };
});

// Combine all initialization in one onMounted
onMounted(async () => {
  // Initialize dimensions first
  updateDimensions();

  // Set up resize observer
  resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      if (entry.target === containerRef.value) {
        updateDimensions();
      }
    }
  });

  if (containerRef.value) {
    resizeObserver.observe(containerRef.value);
  }

  // Add window resize listener as backup
  window.addEventListener('resize', updateDimensions);

  // Force an initial resize after a short delay
  await nextTick();
  updateDimensions();

  try {
    // Get params from URL
    const params = new URLSearchParams(window.location.search);
    serverUrl.value = params.get('server') || '';
    deviceId.value = params.get('device') || '';

    if (!serverUrl.value || !deviceId.value) {
      throw new Error('Missing required parameters: server and device');
    }

    // Modified fetch to handle CORS
    const response = await fetch(`${serverUrl.value}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
        'Access-Control-Allow-Origin': '*',
      },
      mode: 'cors',
      body: JSON.stringify({
        command: 'List',
        module: 'DeviceManager',
      }),
    }).catch((err) => {
      console.warn('Fetch failed, using mock device:', err);
      return {
        ok: true,
        json: () =>
          Promise.resolve({
            DeviceInfo: [
              {
                id: deviceId.value,
                device_type: route.params.type?.toUpperCase() || 'Ping360',
                status: 'ContinuousMode',
                source: {
                  UdpStream: {
                    ip: new URL(serverUrl.value).hostname,
                    port: new URL(serverUrl.value).port,
                  },
                },
              },
            ],
          }),
      };
    });

    if (!response.ok) {
      throw new Error(`Failed to connect to server: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    const device = data.DeviceInfo?.find((d) => d.id === deviceId.value);

    if (!device) {
      throw new Error(`Device not found: ${deviceId.value}`);
    }

    if (device.device_type.toLowerCase() !== widgetType.value) {
      throw new Error(
        `Device type mismatch: expected ${widgetType.value} but got ${device.device_type}`
      );
    }
    deviceData.value = device;
    isLoading.value = false;
  } catch (err) {
    console.error('Widget initialization error:', err);
    error.value = err.message;
    isLoading.value = false;
  }
});

// Proper cleanup
onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
  }
  window.removeEventListener('resize', updateDimensions);
});
</script>

<style scoped>
.h-full {
  height: 100%;
}

.w-full {
  width: 100%;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
</style>