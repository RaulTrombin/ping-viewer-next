<script setup lang="ts">
import { ref, onMounted } from "vue";

// State for loading control
const loading = ref(true);

// Simulate server readiness (replace with actual check)
const checkServerReady = () => {
  setTimeout(() => {
    loading.value = false; // Hide preloader after 2 seconds (replace with actual check)
  }, 2000);
};

onMounted(() => {
  checkServerReady(); // Check server readiness when component is mounted
});
</script>

<template>
  <!-- Preloading screen -->
  <div v-if="loading" class="preloader">
    <h1>Ping Viewer Next</h1>
    <p>Creating a local server...</p>
  </div>

  <!-- Main content (show after loading is complete) -->
  <div v-else class="container">
    <h1>Ping Viewer Next</h1>
    <p>Server is ready!</p>
  </div>
</template>

<style scoped>
/* Ensure preloader and container use the full screen */
.preloader, .container {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  width: 100vw;
  height: 100vh;
  margin: 0;
  padding: 0;
  background-color: #f0f0f0;
}

h1 {
  font-size: 3rem;
  margin-bottom: 1rem;
}

p {
  font-size: 1.5rem;
  color: #666;
}

/* Prevent unwanted margins or scrollbars */
html, body {
  margin: 0;
  padding: 0;
  overflow: hidden;
}
</style>
