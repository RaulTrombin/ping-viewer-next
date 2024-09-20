<template>
  <div class="aspect-square flex justify-center items-center relative">
    <div id="menu-icon" class="absolute top-2 right-2 bg-black bg-opacity-70 p-2 rounded cursor-pointer text-white z-20" @click="toggleControls">
      ☰
    </div>
    <div v-if="showControls" id="controls" class="absolute top-2 left-2 bg-blue-600 bg-opacity-70 p-2 rounded z-10">
      <div v-if="!isConnected">
        <input
          id="hostAddress"
          type="text"
          v-model="hostAddress"
          placeholder="Enter host address"
          class="p-2 mb-2 rounded border border-gray-300 w-full"
        />
        <button @click="listDevices" class="bg-blue-500 text-white py-1 px-2 rounded hover:bg-blue-700">
          List Devices
        </button>
        <select v-if="availableDevices.length > 0" v-model="selectedDevice" class="p-2 my-2 rounded border border-gray-300 w-full">
          <option v-for="device in availableDevices" :value="device.id">
            {{ device.id }}
          </option>
        </select>
        <button @click="connectWebSocket" class="bg-blue-500 text-white py-1 px-2 rounded hover:bg-blue-700">
          Connect
        </button>
      </div>
      <div v-else>
        <button @click="disconnectWebSocket" class="bg-blue-500 text-white py-1 px-2 rounded hover:bg-blue-700">
          Disconnect
        </button>
      </div>
    </div>
    <canvas ref="canvas" class="aspect-square w-full h-full bg-transparent"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';

const canvas = ref<HTMLCanvasElement | null>(null);
const showControls = ref(false);
const availableDevices = ref<{ id: string, source: { UdpStream: { ip: string, port: number } } }[]>([]);
const selectedDevice = ref('');
let isConnected = ref(false);
const hostAddress = ref(window.location.host);

let gl: WebGLRenderingContext | null = null;
let shaderProgram: WebGLProgram | null = null;
let texture: WebGLTexture | null = null;
let socket: WebSocket | null = null;
let lastAngle = 0;

const NUM_LINES = 400;
const LINE_LENGTH = 1200;
const intensityData = new Uint8Array(NUM_LINES * LINE_LENGTH);
let currentAngle = 0;

const vsSource = `
  attribute vec4 aVertexPosition;
  attribute vec2 aTextureCoord;
  varying vec2 vTextureCoord;
  void main(void) {
    gl_Position = aVertexPosition;
    vTextureCoord = aTextureCoord;
  }
`;

const fsSource = `
precision highp float;
varying vec2 vTextureCoord;
uniform sampler2D uSampler;

void main(void) {
  vec2 polar = vTextureCoord;
  float angle = atan(polar.y - 0.5, polar.x - 0.5) + 3.14159/2.0;
  float radius = length(polar - 0.5) * 2.0;

  if (radius > 1.0) {
    gl_FragColor = vec4(0.1, 0.1, 0.1, 0.0); // Transparent background
  } else {
    float texAngle = (angle + 3.14159) / (2.0 * 3.14159);
    if (texAngle > 1.0) {
      texAngle -= 1.0;
    }
    float intensity = texture2D(uSampler, vec2(radius, texAngle)).r;
    gl_FragColor = vec4(intensity, intensity, intensity, 1.0); // Opaque color
  }
}
`;

const initWebGL = () => {
  const canvasElement = canvas.value;
  if (!canvasElement) return;

  gl = canvasElement.getContext('webgl');
  if (!gl) {
    console.error('Unable to initialize WebGL.');
    return;
  }

  gl.clearColor(0.0, 0.0, 0.0, 0.0);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA); 

  shaderProgram = initShaderProgram(gl, vsSource, fsSource);
  setupBuffers();
  setupTexture();
  resizeCanvas();
};

const loadShader = (gl: WebGLRenderingContext, type: number, source: string): WebGLShader | null => {
  const shader = gl.createShader(type);
  if (!shader) {
    console.error('Unable to create shader.');
    return null;
  }

  gl.shaderSource(shader, source);
  gl.compileShader(shader);

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.error('An error occurred compiling the shaders: ' + gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
    return null;
  }

  return shader;
};

const initShaderProgram = (gl: WebGLRenderingContext, vsSource: string, fsSource: string): WebGLProgram | null => {
  const vertexShader = loadShader(gl, gl.VERTEX_SHADER, vsSource);
  const fragmentShader = loadShader(gl, gl.FRAGMENT_SHADER, fsSource);

  if (!vertexShader || !fragmentShader) {
    console.error('Failed to load shaders.');
    return null;
  }

  const shaderProgram = gl.createProgram();
  if (!shaderProgram) {
    console.error('Unable to create shader program.');
    return null;
  }

  gl.attachShader(shaderProgram, vertexShader);
  gl.attachShader(shaderProgram, fragmentShader);
  gl.linkProgram(shaderProgram);

  if (!gl.getProgramParameter(shaderProgram, gl.LINK_STATUS)) {
    console.error('Unable to initialize the shader program: ' + gl.getProgramInfoLog(shaderProgram));
    return null;
  }

  return shaderProgram;
};

const setupBuffers = () => {
  if (!gl || !shaderProgram) return;

  const positionBuffer = gl.createBuffer();
  if (!positionBuffer) return;

  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  const positions = [
    -1.0,  1.0,
     1.0,  1.0,
    -1.0, -1.0,
     1.0, -1.0,
  ];
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(positions), gl.STATIC_DRAW);

  const textureCoordBuffer = gl.createBuffer();
  if (!textureCoordBuffer) return;

  gl.bindBuffer(gl.ARRAY_BUFFER, textureCoordBuffer);
  const textureCoordinates = [
    0.0,  0.0,
    1.0,  0.0,
    0.0,  1.0,
    1.0,  1.0,
  ];
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(textureCoordinates), gl.STATIC_DRAW);

  const vertexPosition = gl.getAttribLocation(shaderProgram, 'aVertexPosition');
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.vertexAttribPointer(vertexPosition, 2, gl.FLOAT, false, 0, 0);
  gl.enableVertexAttribArray(vertexPosition);

  const textureCoord = gl.getAttribLocation(shaderProgram, 'aTextureCoord');
  gl.bindBuffer(gl.ARRAY_BUFFER, textureCoordBuffer);
  gl.vertexAttribPointer(textureCoord, 2, gl.FLOAT, false, 0, 0);
  gl.enableVertexAttribArray(textureCoord);
};

const setupTexture = () => {
  if (!gl) return;

  texture = gl.createTexture();
  if (!texture) return;

  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
};

const updateTexture = () => {
  if (!gl || !texture) return;

  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE, LINE_LENGTH, NUM_LINES, 0, gl.LUMINANCE, gl.UNSIGNED_BYTE, intensityData);
};

const render = () => {
  if (!gl || !shaderProgram) return;

  gl.clear(gl.COLOR_BUFFER_BIT);

  gl.useProgram(shaderProgram);

  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.uniform1i(gl.getUniformLocation(shaderProgram, 'uSampler'), 0);

  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uAngle'), currentAngle);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uNumLines'), NUM_LINES);

  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
};

const updateSonarData = (angle: number, newData: Uint8Array) => {
  const lineIndex = angle;
  const start = lineIndex * LINE_LENGTH;
  intensityData.set(newData, start);
  currentAngle = angle;
  updateTexture();
  render();
};

const listDevices = () => {
  if (socket) {
    socket.close();
  }

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = hostAddress.value.trim();

  socket = new WebSocket(`${protocol}//${host}/ws`);

  if (socket) {
    socket.addEventListener('open', () => {
      console.log('WebSocket connected.');
      if (socket) {
        socket.send(JSON.stringify({ command: 'List', module: 'DeviceManager' }));
      }
    });

    const initialDeviceInfoReceived = ref(false);

socket.addEventListener('message', (event) => {
  try {
    const message = JSON.parse(event.data);

    if (!initialDeviceInfoReceived.value) {
      if (Array.isArray(message.DeviceInfo)) {
        availableDevices.value = message.DeviceInfo.filter((device: any) => device.device_type === 'Ping360');
        console.log("DeviceInfo received:", availableDevices.value);
        initialDeviceInfoReceived.value = true;
        socket.close()
        return
      } else {
        console.warn('DeviceInfo is not an array or is undefined:', message.DeviceInfo);
      }
    } else {
      console.log('Subsequent message received but ignored:', message);
    }

  } catch (error) {
    console.error('Error parsing WebSocket message:', error);
  }
});

    socket.addEventListener('close', () => {
      console.log('WebSocket closed.');
      isConnected.value = false;
    });

    socket.addEventListener('error', (error) => {
      console.error('WebSocket error:', error);
      isConnected.value = false;
    });
  }
};

const connectWebSocket = () => {
  if (!selectedDevice.value) {
    alert('Please select a device.');
    return;
  }

  if (socket) {
    socket.close();
  }

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = hostAddress.value.trim();

  socket = new WebSocket(`${protocol}//${host}/ws?device_number=${selectedDevice.value}`);

  if (socket) {
    socket.addEventListener('open', () => {
      console.log('WebSocket connected.');
      isConnected.value = true;
    });

    socket.addEventListener('message', (event) => {
      const message = JSON.parse(event.data);
      const deviceData = message.DeviceMessage.PingMessage.Ping360.DeviceData;
      const angle = deviceData.angle;
      const newIntensityData = deviceData.data.map((value: any) => value);
      if (angle - lastAngle > 1) {
        console.log('Angle jump detected:', lastAngle, angle);
      }
      lastAngle = angle;
      console.log('Received sonar data at angle:', angle);
      updateSonarData(angle, newIntensityData);
    });

    socket.addEventListener('close', () => {
      console.log('WebSocket closed.');
      isConnected.value = false;
    });

    socket.addEventListener('error', (error) => {
      console.error('WebSocket error:', error);
      isConnected.value = false;
    });
  }
};

const disconnectWebSocket = () => {
  if (socket) {
    socket.close();
    isConnected.value = false;
  }
};

const resizeCanvas = () => {
  if (canvas.value) {
    canvas.value.width = canvas.value.clientWidth;
    canvas.value.height = canvas.value.clientHeight;
    if (gl) {
      gl.viewport(0, 0, canvas.value.width, canvas.value.height);
      render();
    }
  }
};

const toggleControls = () => {
  showControls.value = !showControls.value;
};

const handleClickOutside = (event: MouseEvent) => {
  const controlsElement = document.getElementById('controls');
  const menuIconElement = document.getElementById('menu-icon');
  if (controlsElement && menuIconElement &&
      !controlsElement.contains(event.target as Node) &&
      !menuIconElement.contains(event.target as Node)) {
    showControls.value = false;
  }
};

onMounted(() => {
  nextTick(() => {
    initWebGL();
    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
    document.addEventListener('click', handleClickOutside);
  });
});

onUnmounted(() => {
  if (socket) {
    socket.close();
  }
  window.removeEventListener('resize', resizeCanvas);
  document.removeEventListener('click', handleClickOutside);
});
</script>