<template>
  <div class="bg-gray-100 flex min-h-screen p-6">
    <div class="max-w-3xl mx-auto bg-white shadow-lg rounded-lg overflow-hidden">
      <div class="bg-gray-800 text-white py-4 px-6 flex justify-between items-center">
        <h2 class="text-2xl font-bold">Transducer Settings</h2>
        <button @click="toggleSettingsMenu"
          class="bg-gray-700 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded float-right">
          {{ settingsMenuVisible ? 'Hide Settings' : '☰' }}
        </button>
      </div>

      <div class="p-6 space-y-6">
        <div v-if="settingsMenuVisible" class="bg-gray-100 p-4 rounded-lg space-y-4">
          <div class="flex flex-col md:flex-row md:items-center space-y-2 md:space-y-0 md:space-x-4">
            <label class="text-sm font-medium text-gray-700">Server Address</label>
            <input v-model="serverAddress"
              class="text-black flex-grow mt-1 block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
              placeholder="Enter server address" />
            <button @click="updateAddress"
              class="mt-2 md:mt-0 bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded">Update
              Address</button>
          </div>

          <div class="flex flex-col md:flex-row md:items-center space-y-2 md:space-y-0 md:space-x-4">
            <label class="text-sm font-medium text-gray-700">UUID</label>
            <select v-model="selectedUuid"
              class="text-black flex-grow mt-1 block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500">
              <option disabled value="">Select UUID</option>
              <option v-for="device in filteredDevices" :key="device.id" :value="device.id">{{ device.id }}</option>
            </select>
          </div>
        </div>

        <div v-for="field in fields" :key="field.label" class="space-y-2">
          <label :title="field.description" class="block text-sm font-medium text-gray-700">{{ field.label }}</label>
          <div class="flex items-center space-x-4">
            <input type="range" v-model="field.value" :min="field.min" :max="field.max" class="flex-grow"
              @input="updateRange" />
            <input type="number" v-model="field.value" :min="field.min" :max="field.max"
              class="text-black w-20 border border-gray-300 rounded-md shadow-sm py-1 px-2 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
              @input="updateRange" />
            <span class="text-sm text-gray-500">{{ field.unit }}</span>
          </div>
        </div>

        <div class="space-y-2">
          <label title="Calculated range based on current settings"
            class="block text-sm font-medium text-gray-700">Range</label>
          <div class="flex items-center space-x-4">
            <input type="range" v-model.number="range" :min="rangeMin" :max="rangeMax" class="flex-grow"
              @input="updateSamplePeriod" />
            <input type="number" v-model.number="range" :min="rangeMin" :max="rangeMax"
              class="text-black w-20 border border-gray-300 rounded-md shadow-sm py-1 px-2 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
              @input="updateSamplePeriod" />
            <span class="text-sm text-gray-500">meters</span>
          </div>
        </div>

        <div class="space-y-2">
          <label title="Speed of sound" class="block text-sm font-medium text-gray-700">Speed of Sound</label>
          <div class="flex items-center space-x-4">
            <input type="range" v-model.number="speedOfSound" :min="speedOfSoundMin" :max="speedOfSoundMax"
              class="flex-grow" @input="updateRange" />
            <input type="number" v-model.number="speedOfSound" :min="speedOfSoundMin" :max="speedOfSoundMax"
              class="text-black w-20 border border-gray-300 rounded-md shadow-sm py-1 px-2 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
              @input="updateRange" />
            <span class="text-sm text-gray-500">m/s</span>
          </div>
        </div>

        <button @click="saveSettings"
          class="w-full bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline">
          Save Settings
        </button>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  data() {
    return {
      serverAddress: localStorage.getItem('serverAddress') || window.location.protocol + '//' + window.location.host,
      selectedUuid: localStorage.getItem('selectedUuid') || "",
      devices: [],
      filteredDevices: [],
      settingsMenuVisible: false,
      fields: [
        { label: "Gain Setting", value: 0, min: 0, max: 2, description: "Analog gain setting (0 = low, 1 = normal, 2 = high)" },
        { label: "Transmit Duration", value: 32, min: 1, max: 1000, description: "Acoustic transmission duration (1~1000 us)", unit: "µs" },
        { label: "Sample Period", value: 80, min: 80, max: 40000, description: "Time interval between signal samples (80 to 40000)", unit: "25ns" },
        { label: "Transmit Frequency", value: 740, min: 500, max: 1000, description: "Operating frequency (500~1000 kHz)", unit: "kHz" },
        { label: "Number of Samples", value: 1200, min: 200, max: 1200, description: "Number of samples per reflected signal (200~1200)" },
      ],
      range: 0,
      rangeMin: 1.8,
      rangeMax: 900,
      speedOfSound: 1500,
      speedOfSoundMin: 1400,
      speedOfSoundMax: 1600,
    };
  },
  mounted() {
    this.fetchDevices();
    this.updateRange();
  },
  methods: {
    async fetchDevices() {
      try {
        const response = await fetch(`${this.serverAddress}/device_manager/List`, {
          headers: {
            "accept": "application/json"
          }
        });
        const data = await response.json();
        this.devices = data.DeviceInfo;
        this.filteredDevices = this.devices.filter(device => device.device_type === "Ping360");

        // Check if the cached UUID is still available
        if (this.selectedUuid && !this.filteredDevices.some(device => device.id === this.selectedUuid)) {
          this.selectedUuid = this.filteredDevices.length > 0 ? this.filteredDevices[0].id : "";
        } else if (!this.selectedUuid && this.filteredDevices.length > 0) {
          this.selectedUuid = this.filteredDevices[0].id;
        }

        localStorage.setItem('selectedUuid', this.selectedUuid);
      } catch (error) {
        console.error('Error fetching devices:', error);
      }
    },
    toggleSettingsMenu() {
      this.settingsMenuVisible = !this.settingsMenuVisible;
    },
    updateAddress() {
      localStorage.setItem('serverAddress', this.serverAddress);
      this.fetchDevices();
    },
    updateDependentFields(changedField) {
      if (changedField.label === "Sample Period" || changedField.label === "Number of Samples") {
        this.updateRange();
      }
    },
    updateRange() {
      const samplePeriod = this.fields.find(f => f.label === "Sample Period").value * 25e-9; // Convert to seconds
      const numberOfSamples = this.fields.find(f => f.label === "Number of Samples").value;
      this.range = Number((samplePeriod * numberOfSamples * this.speedOfSound / 2).toFixed(2));
    },
    updateSamplePeriod() {
      const numberOfSamples = this.fields.find(f => f.label === "Number of Samples").value;
      const newSamplePeriod = (this.range * 2) / (numberOfSamples * this.speedOfSound);
      const samplePeriodField = this.fields.find(f => f.label === "Sample Period");
      samplePeriodField.value = Math.round(newSamplePeriod / 25e-9); // Convert back to 25ns increments

      samplePeriodField.value = Math.max(samplePeriodField.min, Math.min(samplePeriodField.value, samplePeriodField.max));
    },
    async saveSettings() {
      if (!this.selectedUuid) {
        alert("Please select a UUID.");
        return;
      }

      const url = `${this.serverAddress}/device_manager/request`;

      const command1 = {
        module: "DeviceManager",
        command: "DisableContinuousMode",
        payload: { uuid: this.selectedUuid }
      };

      const command2 = {
        command: "Ping",
        module: "DeviceManager",
        payload: {
          device_request: {
            Ping360: {
              Transducer: this.fields.reduce((acc, field) => {
                acc[field.label.toLowerCase().replace(/ /g, '_')] = Number(field.value);
                return acc;
              }, { mode: 1, transmit: 1, angle: 0, reserved: 0 })
            }
          },
          uuid: this.selectedUuid
        }
      };

      const command3 = {
        module: "DeviceManager",
        command: "EnableContinuousMode",
        payload: { uuid: this.selectedUuid }
      };

      const sendCommand = (command) => {
        return fetch(url, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(command),
        })
          .then(response => response.json())
          .catch(error => {
            console.error('Error:', error);
            return error;
          });
      };

      const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));

      try {
        //   console.log("Sending command 1:", JSON.stringify(command1, null, 2));
        await sendCommand(command1);
        await delay(500);

        //   console.log("Sending command 2:", JSON.stringify(command2, null, 2));
        this.response = await sendCommand(command2);
        await delay(500);

        //   console.log("Sending command 3:", JSON.stringify(command3, null, 2));
        this.response = await sendCommand(command3);
      } catch (error) {
        console.error('Error:', error);
        this.response = error;
      }
    },
  },
  watch: {
    selectedUuid(newValue) {
      localStorage.setItem('selectedUuid', newValue);
    }
  }
};
</script>