import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useNotificationStore = defineStore('notifications', () => {
  const notifications = ref([]);
  const nextId = ref(1);

  const addNotification = (notification) => {
    const id = nextId.value++;
    notifications.value.push({
      id,
      ...notification,
      timestamp: new Date(),
    });

    // Auto-remove notification after 5 seconds
    setTimeout(() => {
      removeNotification(id);
    }, 5000);
  };

  const removeNotification = (id) => {
    const index = notifications.value.findIndex(n => n.id === id);
    if (index !== -1) {
      notifications.value.splice(index, 1);
    }
  };

  const clearNotifications = () => {
    notifications.value = [];
  };

  return {
    notifications,
    addNotification,
    removeNotification,
    clearNotifications,
  };
}); 