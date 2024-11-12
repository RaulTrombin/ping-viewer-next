<template>
	<div class="flex flex-col h-full">
	  <div class="flex items-center gap-2 mb-2">
		<DataRecorder
		  ref="dataRecorder"
		  :device="device"
		  @recording-complete="handleRecordingComplete"
		/>
		<v-btn
		  icon
		  :color="isFreeze ? 'error' : 'primary'"
		  @click="toggleFreeze"
		  class="elevation-4"
		  size="large"
		>
		  <v-icon>{{ isFreeze ? 'mdi-play' : 'mdi-pause' }}</v-icon>
		</v-btn>
		<Ping360Settings
		  ref="settingsRef"
		  :server-url="getServerUrl(websocketUrl)"
		  :device-id="device.id"
		  :initial-angles="{ startAngle: startAngle, endAngle: endAngle }"
		  @update:angles="handleAngleUpdate"
		  @rangeChange="handleRangeChange"
		/>
	  </div>

	  <Ping360
		:measurement="displayMeasurement"
		:angle="displayAngle"
		:colorPalette="colorPalette"
		:lineColor="lineColor"
		:lineWidth="lineWidth"
		:maxDistance="currentRange"
		:numMarkers="numMarkers"
		:showRadiusLines="showRadiusLines"
		:showMarkers="showMarkers"
		:radiusLineColor="radiusLineColor"
		:markerColor="markerColor"
		:radiusLineWidth="radiusLineWidth"
		:debug="debug"
		:startAngle="startAngle"
		:endAngle="endAngle"
		:yaw_angle="yawAngle"
		v-bind="$attrs"
	  />
	</div>
  </template>

  <script setup>
import { inject, onMounted, onUnmounted, ref, watch } from "vue";
import DataRecorder from "../DataRecorder.vue";
import Ping360 from "./Ping360.vue";
import Ping360Settings from "./Ping360Settings.vue";

const props = defineProps({
	device: {
		type: Object,
		required: true,
	},
	websocketUrl: {
		type: String,
		required: true,
	},
	colorPalette: {
		type: String,
		required: true,
	},
	lineColor: {
		type: String,
		default: "red",
	},
	lineWidth: {
		type: Number,
		default: 0.5,
	},
	maxDistance: {
		type: Number,
		default: 300,
	},
	numMarkers: {
		type: Number,
		default: 5,
	},
	showRadiusLines: {
		type: Boolean,
		default: true,
	},
	showMarkers: {
		type: Boolean,
		default: true,
	},
	radiusLineColor: {
		type: String,
		default: "green",
	},
	markerColor: {
		type: String,
		default: "green",
	},
	radiusLineWidth: {
		type: Number,
		default: 0.5,
	},
	debug: {
		type: Boolean,
		default: false,
	},
});

const liveMeasurement = ref(null);
const liveAngle = ref(0);
const displayMeasurement = ref(null);
const displayAngle = ref(0);
const debug = ref(false);
const currentRange = ref(props.maxDistance);
const startAngle = ref(0);
const endAngle = ref(360);
const connectionStatus = ref("Disconnected");
const dataRecorder = ref(null);
const socket = ref(null);
const settingsRef = ref(null);
const isFreeze = ref(false);

const yawAngle = inject("yawAngle", ref(0));

const getServerUrl = (wsUrl) => {
	try {
		const url = new URL(wsUrl);
		return `http${url.protocol === "wss:" ? "s" : ""}://${url.host}`;
	} catch (error) {
		console.error("Error parsing WebSocket URL:", error);
		return "";
	}
};

const toggleFreeze = () => {
	isFreeze.value = !isFreeze.value;
	if (!isFreeze.value) {
		displayMeasurement.value = liveMeasurement.value;
		displayAngle.value = liveAngle.value;
	}
};

const { handleRecordingComplete: notifyRecording } = inject("recordings");

const handleRecordingComplete = (recordingData) => {
	if (notifyRecording) {
		const recordingWithSettings = {
			...recordingData,
			settings: {
				startAngle: startAngle.value,
				endAngle: endAngle.value,
				currentRange: currentRange.value,
				yawAngle: yawAngle.value,
			},
		};
		notifyRecording(recordingWithSettings);
	}
};

function gradiansToDegrees(gradians) {
	if (gradians === 399) {
		return 360;
	}
	return Math.round((gradians * 360) / 400);
}

const connectWebSocket = () => {
	if (socket.value) return;

	socket.value = new WebSocket(props.websocketUrl);

	socket.value.onopen = () => {
		console.log("Ping360 WebSocket connected");
		connectionStatus.value = "Connected";
	};

	socket.value.onmessage = (event) => {
		try {
			const parsedData = JSON.parse(event.data);
			if (props.debug) {
				console.log("Received raw data:", parsedData);
			}

			if (
				parsedData.DeviceConfig?.ConfigAcknowledge?.modify?.SetPing360Config
			) {
				const config =
					parsedData.DeviceConfig.ConfigAcknowledge.modify.SetPing360Config;

				const SAMPLE_PERIOD_TICK_DURATION = 25e-9;
				currentRange.value = Math.round(
					(config.sample_period *
						SAMPLE_PERIOD_TICK_DURATION *
						config.number_of_samples *
						1500) /
						2,
				);

				if (config.start_angle === 0 && config.stop_angle === 399) {
					startAngle.value = 0;
					endAngle.value = 360;
				} else {
					startAngle.value =
						(gradiansToDegrees(config.start_angle) + 180) % 360;
					endAngle.value = (gradiansToDegrees(config.stop_angle) + 180) % 360;
				}

				return;
			}

			const ping360Data = parsedData?.DeviceMessage?.PingMessage?.Ping360;
			if (!ping360Data) return;

			const messageData = ping360Data.DeviceData || ping360Data.AutoDeviceData;
			if (!messageData || messageData.angle === undefined || !messageData.data)
				return;

			liveMeasurement.value = {
				angle: messageData.angle,
				data: new Uint8Array(messageData.data),
			};
			liveAngle.value = messageData.angle;

			if (!isFreeze.value) {
				displayMeasurement.value = liveMeasurement.value;
				displayAngle.value = liveAngle.value;
			}

			dataRecorder.value?.recordData({
				angle: messageData.angle,
				data: new Uint8Array(messageData.data),
			});

			if (props.debug) {
				console.log("Processed data:", {
					angle: liveAngle.value,
					dataLength: liveMeasurement.value.data.length,
					yawAngle: yawAngle.value,
				});
			}
		} catch (error) {
			console.error("Error parsing Ping360 WebSocket data:", error);
		}
	};

	socket.value.onerror = (error) => {
		console.error("Ping360 WebSocket error:", error);
		connectionStatus.value = "Error";
	};

	socket.value.onclose = () => {
		console.log("Ping360 WebSocket disconnected");
		connectionStatus.value = "Disconnected";
		socket.value = null;
		setTimeout(connectWebSocket, 5000);
	};
};

const disconnectWebSocket = () => {
	if (socket.value) {
		socket.value.close();
		socket.value = null;
	}
};

const handleAngleUpdate = ({ startAngle: newStart, endAngle: newEnd }) => {
	startAngle.value = newStart;
	endAngle.value = newEnd;
};

const handleRangeChange = (newRange) => {
	currentRange.value = newRange;
};

watch(
	() => props.websocketUrl,
	(newUrl, oldUrl) => {
		if (newUrl !== oldUrl) {
			disconnectWebSocket();
			connectWebSocket();
		}
	},
);

watch(yawAngle, (newYaw) => {
	if (props.debug) {
		console.log("Yaw angle updated:", newYaw);
	}
});

onMounted(async () => {
	console.log("Ping360Loader mounted with props:", props);
	await settingsRef.value.fetchCurrentSettings();
	connectWebSocket();
});

onUnmounted(() => {
	disconnectWebSocket();
});
</script>