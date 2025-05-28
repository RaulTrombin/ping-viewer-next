import { ref, onMounted, onUnmounted } from 'vue';
import { useNotificationStore } from '@/stores/notificationStore';

export function useRecordingSessions(serverUrl) {
  const recordingSessions = ref(new Map());
  const ws = ref(null);
  const isConnected = ref(false);
  const error = ref(null);
  const notificationStore = useNotificationStore();
  const reconnectAttempts = ref(0);
  const maxReconnectAttempts = 5;
  const reconnectTimeout = ref(null);
  const lastNotificationTime = ref(0);
  const NOTIFICATION_COOLDOWN = 30000; // 30 seconds between notifications

  const getWebSocketUrl = (url) => {
    try {
      // Remove any existing protocol
      const cleanUrl = url.replace(/^https?:\/\//, '');
      // Ensure we have a valid URL
      const wsUrl = `ws://${cleanUrl}/ws/recording`;
      console.log('Attempting to connect to WebSocket:', wsUrl);
      return wsUrl;
    } catch (err) {
      console.error('Error constructing WebSocket URL:', err);
      throw new Error('Invalid server URL format');
    }
  };

  const showNotification = (notification) => {
    const now = Date.now();
    if (now - lastNotificationTime.value > NOTIFICATION_COOLDOWN) {
      notificationStore.addNotification(notification);
      lastNotificationTime.value = now;
    }
  };

  const connect = () => {
    if (ws.value?.readyState === WebSocket.OPEN) {
      console.log('WebSocket already connected');
      return; // Already connected
    }

    try {
      const wsUrl = getWebSocketUrl(serverUrl);
      console.log('Creating WebSocket connection to:', wsUrl);
      
      ws.value = new WebSocket(wsUrl);
      
      ws.value.onopen = () => {
        console.log('Recording sessions WebSocket connected successfully');
        isConnected.value = true;
        error.value = null;
        reconnectAttempts.value = 0;
        showNotification({
          title: 'Connected',
          message: 'Recording service connected successfully',
          icon: 'mdi-wifi',
          color: 'success',
        });
      };

      ws.value.onmessage = (event) => {
        try {
          const session = JSON.parse(event.data);
          console.log('Received recording session:', session);
          
          // Update the recording sessions map
          if (session.device_id) {
            const previousSession = recordingSessions.value.get(session.device_id);
            recordingSessions.value.set(session.device_id, session);

            // Show notification for recording status changes
            if (!previousSession && session.is_active) {
              showNotification({
                title: 'Recording Started',
                message: `Recording started for device ${session.device_id}`,
                icon: 'mdi-record',
                color: 'success',
              });
            } else if (previousSession?.is_active && !session.is_active) {
              showNotification({
                title: 'Recording Stopped',
                message: `Recording stopped for device ${session.device_id}`,
                icon: 'mdi-stop',
                color: 'error',
              });
            }
          }
        } catch (err) {
          console.error('Error parsing recording session:', err);
          error.value = 'Failed to parse recording session data';
        }
      };

      ws.value.onerror = (event) => {
        console.error('Recording sessions WebSocket error:', {
          event,
          readyState: ws.value?.readyState,
          url: wsUrl
        });
        error.value = 'WebSocket connection error';
        isConnected.value = false;
      };

      ws.value.onclose = (event) => {
        console.log('Recording sessions WebSocket disconnected:', {
          code: event.code,
          reason: event.reason,
          wasClean: event.wasClean,
          url: wsUrl
        });
        isConnected.value = false;

        // Only attempt to reconnect if we haven't exceeded max attempts
        if (reconnectAttempts.value < maxReconnectAttempts) {
          reconnectAttempts.value++;
          const delay = Math.min(1000 * Math.pow(2, reconnectAttempts.value), 30000); // Exponential backoff, max 30s
          
          console.log(`Attempting to reconnect in ${delay}ms (attempt ${reconnectAttempts.value}/${maxReconnectAttempts})`);
          
          showNotification({
            title: 'Connection Lost',
            message: `Recording service connection lost. Attempting to reconnect (${reconnectAttempts.value}/${maxReconnectAttempts})...`,
            icon: 'mdi-wifi-off',
            color: 'warning',
          });

          reconnectTimeout.value = setTimeout(connect, delay);
        } else {
          console.error('Max reconnection attempts reached');
          showNotification({
            title: 'Connection Failed',
            message: 'Failed to connect to recording service after multiple attempts. Please check if the server is running and refresh the page.',
            icon: 'mdi-wifi-off',
            color: 'error',
          });
        }
      };
    } catch (err) {
      console.error('Error creating WebSocket connection:', err);
      error.value = 'Failed to create WebSocket connection';
      isConnected.value = false;
      showNotification({
        title: 'Connection Error',
        message: 'Failed to connect to recording service. Please check if the server is running.',
        icon: 'mdi-wifi-off',
        color: 'error',
      });
    }
  };

  const disconnect = () => {
    if (ws.value) {
      console.log('Disconnecting WebSocket');
      ws.value.close();
      ws.value = null;
    }
    if (reconnectTimeout.value) {
      clearTimeout(reconnectTimeout.value);
      reconnectTimeout.value = null;
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
    connect();
  });

  onUnmounted(() => {
    disconnect();
  });

  return {
    recordingSessions,
    isConnected,
    error,
    isDeviceRecording,
    getRecordingSession,
    connect,
    disconnect
  };
}