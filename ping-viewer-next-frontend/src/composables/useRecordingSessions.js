import { ref, onMounted, onUnmounted } from 'vue';
import { useNotificationStore } from '@/stores/notificationStore';

// WebSocket manager singleton
const createWebSocketManager = () => {
  let ws = null;
  let reconnectTimeout = null;
  let reconnectAttempts = 0;
  const maxReconnectAttempts = 5;
  const listeners = new Set();
  let currentUrl = null;

  const connect = (url) => {
    if (ws?.readyState === WebSocket.OPEN) {
      return;
    }

    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }

    try {
      currentUrl = url;
      const wsUrl = `ws://${url.replace(/^https?:\/\//, '')}/ws/recording`;
      console.log('Creating WebSocket connection to:', wsUrl);

      ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        console.log('Recording sessions WebSocket connected successfully');
        reconnectAttempts = 0;
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          listeners.forEach(listener => listener(data));
        } catch (err) {
          console.error('Error parsing WebSocket message:', err);
        }
      };

      ws.onerror = (event) => {
        console.error('WebSocket error:', event);
      };

      ws.onclose = () => {
        if (reconnectAttempts < maxReconnectAttempts && currentUrl) {
          reconnectTimeout = setTimeout(() => {
            reconnectAttempts++;
            connect(currentUrl);
          }, 5000);
        }
      };
    } catch (err) {
      console.error('Error creating WebSocket connection:', err);
    }
  };

  const disconnect = () => {
    if (ws) {
      ws.close();
      ws = null;
    }
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }
    currentUrl = null;
    listeners.clear();
  };

  const addListener = (listener) => {
    listeners.add(listener);
    // If we have a URL but no connection, try to connect
    if (currentUrl && (!ws || ws.readyState !== WebSocket.OPEN)) {
      connect(currentUrl);
    }
  };

  const removeListener = (listener) => {
    listeners.delete(listener);
    // Only disconnect if there are no more listeners
    if (listeners.size === 0) {
      disconnect();
    }
  };

  return {
    connect,
    disconnect,
    addListener,
    removeListener
  };
};

// Create a single instance of the WebSocket manager
const wsManager = createWebSocketManager();

export function useRecordingSessions(serverUrl, wsManagerInstance = wsManager) {
  const recordingSessions = ref(new Map());
  const isConnected = ref(false);
  const error = ref(null);

  const fetchInitialRecordingStatuses = async () => {
    try {
      const response = await fetch(`${serverUrl}/v1/device_manager/GetAllRecordingStatus`);
      if (!response.ok) {
        throw new Error('Failed to fetch recording statuses');
      }
      const data = await response.json();
      if (data.AllRecordingStatus) {
        data.AllRecordingStatus.forEach(session => {
          recordingSessions.value.set(session.device_id, session);
        });
      }
    } catch (err) {
      console.error('Error fetching initial recording statuses:', err);
      error.value = 'Failed to fetch initial recording statuses';
    }
  };

  const isDeviceRecording = (deviceId) => {
    const session = recordingSessions.value.get(deviceId);
    return session?.is_active || false;
  };

  const getRecordingSession = (deviceId) => {
    return recordingSessions.value.get(deviceId);
  };

  onMounted(() => {
    wsManagerInstance.connect(serverUrl);
    fetchInitialRecordingStatuses();
  });

  onUnmounted(() => {
    // No need to remove listener since we're not adding one
  });

  return {
    isDeviceRecording,
    getRecordingSession,
    recordingSessions,
    isConnected,
    error
  };
}

// Export the WebSocket manager for use in Main.vue
export { wsManager };