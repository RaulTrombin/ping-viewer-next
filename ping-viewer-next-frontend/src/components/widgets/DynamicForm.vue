<template>
  <div class="bg-gray-100 p-6">
    <div class="max-w-3xl mx-auto bg-white shadow-lg rounded-lg overflow-hidden">
      <div class="bg-gray-800 text-white py-4 px-6">
        <h2 class="text-2xl font-bold">Ping Viewer DynamicForm</h2>
      </div>

      <div class="p-6">
        <form @submit.prevent="submitForm" class="space-y-6">
          <div>
            <label for="baseUrl" class="block text-sm font-medium text-gray-700">Base URL:</label>
            <input id="baseUrl" v-model="baseUrl" placeholder="Enter base URL" class="mt-1 block w-full border text-black border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500" />
          </div>

          <div>
            <label for="module" class="block text-sm font-medium text-gray-700">Module:</label>
            <select id="module" v-model="selectedModule" @change="updateCommands" class="mt-1 block w-full border text-black border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500">
              <option value="DeviceManager">DeviceManager</option>
            </select>
          </div>

          <div v-if="selectedModule">
            <label for="command" class="block text-sm font-medium text-gray-700">Command:</label>
            <select id="command" v-model="selectedCommand" @change="updatePayload" class="mt-1 block w-full border text-black border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500">
              <option v-for="command in availableCommands" :key="command" :value="command">
                {{ command }}
              </option>
            </select>
          </div>

          <div v-if="payloadFields.length > 0" class="space-y-4">
            <h3 class="text-lg font-medium text-gray-900">Payload:</h3>
            <div v-for="field in payloadFields" :key="field.path" class="space-y-2">
              <label :for="field.path" class="block text-sm font-medium text-gray-700">{{ field.name }}:</label>
              <template v-if="field.type === 'select'">
                <select :id="field.path" v-model="payload[field.path]" @change="onSelectChange(field)" class="mt-1 block w-full border text-black border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500">
                  <option v-for="option in field.options" :key="option" :value="option">
                    {{ option }}
                  </option>
                </select>
              </template>
              <template v-else>
                <input
                  :id="field.path"
                  v-model="payload[field.path]"
                  :type="field.type"
                  class="mt-1 block w-full border text-black border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                >
              </template>
            </div>
          </div>

          <button type="submit" :disabled="!isFormValid" class="mb-6 bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded">
            Submit
          </button>
        </form>

        <div v-if="response" class="mt-8">
          <h3 class="text-lg font-medium text-gray-900 mb-4">Response:</h3>
          <pre class="bg-gray-100 p-4 rounded-md overflow-x-auto messages-container h-64 overflow-y-auto mb-4">{{ response }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import jsonSchema from '../../schema.json';

export default {
  data() {
    return {
      schema: jsonSchema,
      selectedModule: '',
      selectedCommand: '',
      availableCommands: [],
      payload: {},
      payloadFields: [],
      baseUrl: window.location.protocol + '//' + window.location.host,
      response: null,
    };
  },
  computed: {
    isFormValid() {
      return this.selectedModule && this.selectedCommand;
    },
  },
  methods: {
    updateCommands() {
      this.availableCommands = this.schema.oneOf[0].oneOf.map(item => item.properties.command.enum[0]);
      this.selectedCommand = '';
      this.payload = {};
      this.payloadFields = [];
    },
    async fetchAvailableDevices() {
      try {
        const response = await fetch(`${this.baseUrl}/device_manager/List`);
        const data = await response.json();
        this.availableDevices = data.DeviceInfo.map(device => device.id);
      } catch (error) {
        console.error('Failed to fetch devices:', error);
        this.availableDevices = [];
      }
    },
    async updatePayload() {
      const commandSchema = this.schema.oneOf[0].oneOf.find(
        item => item.properties.command.enum[0] === this.selectedCommand
      );

      if (commandSchema && commandSchema.properties.payload) {
        const payloadRef = commandSchema.properties.payload.$ref;
        const payloadSchema = this.resolveRef(payloadRef);
        this.payloadFields = this.extractPayloadFields(payloadSchema);

        this.payload = {};

        if (this.selectedCommand === 'Create') {
          this.payloadFields.unshift({
            name: 'Device Selection',
            path: 'device_selection',
            type: 'select',
            options: this.schema.definitions.DeviceSelection.enum
          });

          this.payloadFields.push({
            name: 'Source Type',
            path: 'source_type',
            type: 'select',
            default: 'Udp',
            options: ['Udp', 'Serial']
          });
          this.payloadFields = this.payloadFields.filter(f => !f.path.startsWith('source.') && f.path !== '');
        }

const existingIndex = this.payloadFields.findIndex(f => f.path === 'uuid');

if (existingIndex !== -1) {
await this.fetchAvailableDevices();

const newUuidField = {
  name: 'uuid',
  path: 'uuid',
  type: this.availableDevices.length > 0 ? 'select' : 'text',
  options: this.availableDevices.length > 0 ? this.availableDevices : [],
};

  this.payloadFields[existingIndex] = newUuidField;
}


      } else {
        this.payloadFields = [];
        this.payload = {};
      }
    },
    resolveRef(ref) {
      const path = ref.split('/').slice(1);
      let schema = this.schema;
      for (const key of path) {
        if (schema[key] === undefined) {
          console.error(`Unable to resolve reference: ${ref}`);
          return {};
        }
        schema = schema[key];
      }
      return schema;
    },
    extractPayloadFields(schema, parentPath = '') {
      let fields = [];

      if (schema.type === 'object' && schema.properties) {
        for (const [name, prop] of Object.entries(schema.properties)) {
          const path = parentPath ? `${parentPath}.${name}` : name;

          if (prop.$ref) {
            const nestedSchema = this.resolveRef(prop.$ref);
            fields = fields.concat(this.extractPayloadFields(nestedSchema, path));
          } else if (prop.type === 'object') {
            if (prop.oneOf) {
              fields.push({
                name,
                path,
                type: 'select',
                options: prop.oneOf.map(item => item.enum[0] || item.type),
              });

              prop.oneOf.forEach((item, index) => {
                if (item.type === 'object' && item.properties) {
                  const subFields = this.extractPayloadFields(item, `${path}[${index}]`);
                  fields = fields.concat(subFields);
                }
              });
            } else {
              fields = fields.concat(this.extractPayloadFields(prop, path));
            }
          } else if (prop.enum) {
            fields.push({
              name,
              path,
              type: 'select',
              options: prop.enum,
            });
          } else {
            fields.push({
              name,
              path,
              type: prop.type === 'integer' ? 'number' : prop.type,
            });
          }
        }
      } else if (schema.type === 'string' && schema.format === 'uuid') {
        fields.push({
          name: parentPath,
          path: parentPath,
          type: 'text',
        });
      } else if (schema.oneOf) {
        schema.oneOf.forEach((item, index) => {
          if (item.type === 'string') {
            fields.push({
              name: parentPath,
              path: `${parentPath}`,
              type: 'select',
              options: item.enum,
            });
          } else if (item.type === 'object') {
            const index = fields.findIndex(field => field.path === `${parentPath}`);

            if (index !== -1) {
              fields[index].options = fields[index].options.concat(item.required);
            }
            const subFields = this.extractPayloadFields(item, `${parentPath}`);
            fields = fields.concat(subFields);
          }
        });
      }

      return fields;
    },
    onSelectChange(field) {
      if (field.path === 'source_type' ) {
        const sourceType = this.payload[field.path];
        const sourceSchema = this.resolveRef(`#/definitions/SourceSelection`);
        const sourceFields = this.extractPayloadFields(sourceSchema, 'source');

        this.payloadFields = this.payloadFields.filter(f => !f.path.startsWith('source.') && f.path !== '');

        for (let i = 0; i < sourceFields.length; i++) {
          if (sourceFields[i].path.includes(`source.${sourceType}Stream`)) {
            this.payloadFields.push(sourceFields[i]);
          }
        }
      } else if (field.path === 'device_request') {
        const selectedDeviceRequest = this.payload[field.path];
        const pingRequestSchema = this.resolveRef(`#/definitions/PingRequest`);
        const pingRequestFields = this.extractPayloadFields(pingRequestSchema);

        this.payloadFields = this.payloadFields.filter(f => !f.path.startsWith('device_request.'));

        if (['GetSubscriber', 'Upgrade', 'Stop'].includes(selectedDeviceRequest)) {
          this.payloadFields = this.payloadFields.concat(pingRequestFields.filter(f => f.path.startsWith('device_request')));
        } else {
          for (let i = 0; i < pingRequestFields.length; i++) {
            const fieldPath = pingRequestFields[i].path;
            if (fieldPath.startsWith(`device_request.${selectedDeviceRequest}`)) {
              this.payloadFields.push(pingRequestFields[i]);
            }
          }

          let requestSchema;
          if (selectedDeviceRequest === 'Common') {
            requestSchema = this.resolveRef(`#/definitions/PingCommonRequest`);
          } else if (['Ping1D', 'Ping360'].includes(selectedDeviceRequest)) {
            requestSchema = this.resolveRef(`#/definitions/${selectedDeviceRequest}Request`);
          }

          if (requestSchema) {
            const requestOptions = requestSchema.enum || [];
            if (requestOptions.length > 0) {
              this.payloadFields.push({
                name: `${selectedDeviceRequest} Options`,
                path: `device_request.${selectedDeviceRequest}`,
                type: 'select',
                options: requestOptions,
              });
            }
          }
        }
      }
    },
    submitForm() {
      const command = {
        module: this.selectedModule,
        command: this.selectedCommand,
      };

      if (Object.keys(this.payload).length > 0) {
        command.payload = this.buildNestedPayload(this.payload);
      }

      const url = `${this.baseUrl}/device_manager/request`;

      fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(command),
      })
        .then(response => response.json())
        .then(data => {
          this.response = data;
        })
        .catch(error => {
          console.error('Error:', error);
          this.response = error;
        });
    },
    buildNestedPayload(flatPayload) {
      const nestedPayload = {};

      for (const [path, value] of Object.entries(flatPayload)) {
        const keys = path.split('.');
        let current = nestedPayload;

        for (let i = 0; i < keys.length; i++) {
          const key = keys[i];

          if (i === keys.length - 1) {
            if (key === 'device_request') {
              if (typeof current[key] === 'string' && !['Ping1D', 'Ping360', 'Common'].includes(current[key])) {
                current[key] = {};
              }
            } else {
              current[key] = value;
            }
          } else {
            current[key] = current[key] || {};
            current = current[key];
          }
        }
      }

      return nestedPayload;
    }
  },
};
</script>