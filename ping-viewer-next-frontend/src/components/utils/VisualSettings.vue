<template>
	<div>
		<v-tabs v-model="activeSettingsTab">
			<v-tab value="general">
				<v-icon start>mdi-cog</v-icon>
				General
			</v-tab>

			<v-tab value="ping1d">
				<v-icon start>mdi-altimeter</v-icon>
				Ping1D
			</v-tab>

			<v-tab value="ping360">
				<v-icon start>mdi-radar</v-icon>
				Ping360
			</v-tab>

			<v-tab value="presets">
				<v-icon start>mdi-palette-swatch</v-icon>
				Presets
			</v-tab>
		</v-tabs>

		<v-window v-model="activeSettingsTab" class="mt-2">

			<v-window-item value="general">
				<v-card flat>
					<v-card-text>
						<v-list>
							<v-list-subheader>Display</v-list-subheader>
							<v-list-item>
								<SonarColorOptions :initial-palette="localCommonSettings.colorPalette"
									@update:colorPalette="updateColorPalette" />
							</v-list-item>

							<v-list-item>
								<v-list-item-title class="d-flex align-center justify-space-between">
									Dark Mode
									<v-switch :model-value="isDarkMode"
										@update:model-value="$emit('update:isDarkMode', $event)" hide-details
										inset></v-switch>
								</v-list-item-title>
							</v-list-item>
						</v-list>
					</v-card-text>
				</v-card>
			</v-window-item>

			<v-window-item value="ping1d">
				<v-card flat>
					<v-card-text>
						<v-list>
							<v-list-subheader>Display Settings</v-list-subheader>

							<v-list-item>
								<v-text-field v-model.number="localPing1DSettings.columnCount" type="number"
									label="Column Count" hide-details density="compact"></v-text-field>
							</v-list-item>

							<v-list-item>
								<v-text-field v-model.number="localPing1DSettings.tickCount" type="number"
									label="Tick Count" hide-details density="compact"></v-text-field>
							</v-list-item>

							<v-list-subheader>Colors</v-list-subheader>

							<ColorPickerField v-model="localPing1DSettings.depthLineColor" label="Depth Line Color"
								:defaultValue="'#ffeb3b'" />

							<ColorPickerField v-model="localPing1DSettings.depthTextColor" label="Depth Text Color"
								:defaultValue="'#ffeb3b'" />

							<ColorPickerField v-model="localPing1DSettings.currentDepthColor"
								label="Current Depth Color" :defaultValue="'#ffeb3b'" />

							<ColorPickerField v-model="localPing1DSettings.confidenceColor" label="Confidence Color"
								:defaultValue="'#4caf50'" />

							<ColorPickerField v-model="localPing1DSettings.depthArrowColor" label="Depth Arrow Color"
								:defaultValue="'#f44336'" />

							<ColorPickerField v-model="localPing1DSettings.textBackground" label="Text Background Color"
								:defaultValue="'rgba(0, 0, 0, 0.5)'" />

							<v-list-item>
								<v-switch v-model="localPing1DSettings.debug" label="Debug Mode" hide-details
									inset></v-switch>
							</v-list-item>
						</v-list>
					</v-card-text>
				</v-card>
			</v-window-item>

			<v-window-item value="ping360">
				<v-card flat>
					<v-card-text>
						<v-list>
							<v-list-subheader>Display Settings</v-list-subheader>

							<v-list-item>
								<v-text-field v-model.number="localPing360Settings.numMarkers" type="number"
									label="Number of Markers" hide-details density="compact"></v-text-field>
							</v-list-item>

							<v-list-item>
								<v-text-field v-model.number="localPing360Settings.lineWidth" type="number" step="0.1"
									label="Line Width" hide-details density="compact"></v-text-field>
							</v-list-item>

							<v-list-item>
								<v-text-field v-model.number="localPing360Settings.radiusLineWidth" type="number"
									step="0.1" label="Radius Line Width" hide-details density="compact"></v-text-field>
							</v-list-item>

							<v-list-subheader>Display Options</v-list-subheader>

							<v-list-item>
								<v-list-item-title class="d-flex align-center justify-space-between">
									Show Radius Lines
									<v-switch v-model="localPing360Settings.showRadiusLines" hide-details
										inset></v-switch>
								</v-list-item-title>
							</v-list-item>

							<v-list-item>
								<v-list-item-title class="d-flex align-center justify-space-between">
									Show Markers
									<v-switch v-model="localPing360Settings.showMarkers" hide-details inset></v-switch>
								</v-list-item-title>
							</v-list-item>

							<v-list-item>
								<v-switch v-model="localPing360Settings.debug" label="Debug Mode" hide-details
									inset></v-switch>
							</v-list-item>

							<v-list-subheader>Colors</v-list-subheader>

							<ColorPickerField v-model="localPing360Settings.lineColor" label="Line Color"
								:defaultValue="'#f44336'" />

							<ColorPickerField v-model="localPing360Settings.markerColor" label="Marker Color"
								:defaultValue="'#4caf50'" />

							<ColorPickerField v-model="localPing360Settings.radiusLineColor" label="Radius Line Color"
								:defaultValue="'#4caf50'" />
						</v-list>
					</v-card-text>
				</v-card>
			</v-window-item>

			<v-window-item value="presets">
				<v-card flat>
					<v-card-text>
						<div class="d-flex flex-column gap-4">
							<div>
								<div class="text-subtitle-1 mb-2">Accessibility Presets</div>
								<v-select v-model="selectedPreset" :items="accessibilityPresets"
									label="Color Vision Mode" @update:model-value="handlePresetChange"
									variant="outlined" density="comfortable" />
							</div>

							<v-alert v-if="selectedPreset !== 'default'" color="info" variant="tonal" class="mb-2">
								{{ getPresetDescription(selectedPreset) }}
								<div class="text-caption mt-2">
									You can still adjust individual settings in other tabs.
								</div>
							</v-alert>
						</div>
					</v-card-text>
				</v-card>
			</v-window-item>

		</v-window>

		<div class="d-flex justify-end mt-4">
			<v-btn color="error" variant="text" class="mr-2" @click="handleReset">
				Reset
			</v-btn>
			<v-btn color="primary" variant="text" @click="saveSettings">
				Save
			</v-btn>
		</div>
	</div>
</template>

<script setup>
import { reactive, ref, watch } from 'vue';
import SonarColorOptions from '../widgets/SonarColorOptions.vue';
import { colorPalettes } from '../widgets/SonarColorOptions.vue';
import ColorPickerField from './ColorPickerField.vue';

const props = defineProps({
  commonSettings: {
    type: Object,
    required: true,
  },
  ping1DSettings: {
    type: Object,
    required: true,
  },
  ping360Settings: {
    type: Object,
    required: true,
  },
  isDarkMode: {
    type: Boolean,
    required: true,
  },
});

const emit = defineEmits([
  'update:commonSettings',
  'update:ping1DSettings',
  'update:ping360Settings',
  'update:isDarkMode',
  'save',
  'reset',
]);

const activeSettingsTab = ref('general');
const selectedPreset = ref('default');
const localCommonSettings = reactive({ ...props.commonSettings });
const localPing1DSettings = reactive({ ...props.ping1DSettings });
const localPing360Settings = reactive({ ...props.ping360Settings });

const accessibilityPresets = [
  { title: 'Default', value: 'default' },
  { title: 'Deuteranopia (Red-Green)', value: 'deuteranopia' },
  { title: 'Protanopia (Red-Green)', value: 'protanopia' },
  { title: 'Tritanopia (Blue-Yellow)', value: 'tritanopia' },
  { title: 'Monochromacy', value: 'monochromacy' },
  { title: 'High Contrast', value: 'highContrast' },
];

const presetConfigs = {
  default: {
    description: 'Default color settings',
    settings: {
      commonSettings: {
        colorPalette: 'Ocean',
      },
      ping1DSettings: {
        columnCount: 100,
        tickCount: 5,
        depthLineColor: '#ffeb3b',
        depthTextColor: '#ffeb3b',
        currentDepthColor: '#ffeb3b',
        confidenceColor: '#4caf50',
        textBackground: 'rgba(0, 0, 0, 0.5)',
        depthArrowColor: '#f44336',
        debug: false,
      },
      ping360Settings: {
        lineColor: '#f44336',
        lineWidth: 0.5,
        numMarkers: 5,
        showRadiusLines: true,
        showMarkers: true,
        radiusLineColor: '#4caf50',
        markerColor: '#4caf50',
        radiusLineWidth: 0.5,
        debug: false,
      },
    },
  },
  deuteranopia: {
    description: 'Optimized for red-green color blindness (deuteranopia)',
    settings: {
      commonSettings: {
        colorPalette: 'Monochrome Black',
      },
      ping1DSettings: {
        depthLineColor: '#0077BB',
        depthTextColor: '#0077BB',
        currentDepthColor: '#0077BB',
        confidenceColor: '#EE7733',
        textBackground: 'rgba(0, 0, 0, 0.7)',
        depthArrowColor: '#EE7733',
      },
      ping360Settings: {
        lineColor: '#EE7733',
        radiusLineColor: '#0077BB',
        markerColor: '#0077BB',
      },
    },
  },
  protanopia: {
    description: 'Optimized for red-green color blindness (protanopia)',
    settings: {
      commonSettings: {
        colorPalette: 'Monochrome Black',
      },
      ping1DSettings: {
        depthLineColor: '#0077BB',
        depthTextColor: '#0077BB',
        currentDepthColor: '#0077BB',
        confidenceColor: '#CCBB44',
        textBackground: 'rgba(0, 0, 0, 0.7)',
        depthArrowColor: '#CCBB44',
      },
      ping360Settings: {
        lineColor: '#CCBB44',
        radiusLineColor: '#0077BB',
        markerColor: '#0077BB',
      },
    },
  },
  tritanopia: {
    description: 'Optimized for blue-yellow color blindness',
    settings: {
      commonSettings: {
        colorPalette: 'Monochrome Black',
      },
      ping1DSettings: {
        depthLineColor: '#FF99AA',
        depthTextColor: '#FF99AA',
        currentDepthColor: '#FF99AA',
        confidenceColor: '#44BB99',
        textBackground: 'rgba(0, 0, 0, 0.7)',
        depthArrowColor: '#44BB99',
      },
      ping360Settings: {
        lineColor: '#FF99AA',
        radiusLineColor: '#44BB99',
        markerColor: '#44BB99',
      },
    },
  },
  monochromacy: {
    description: 'Monochrome mode using high-contrast patterns',
    settings: {
      commonSettings: {
        colorPalette: 'Monochrome Black',
      },
      ping1DSettings: {
        depthLineColor: '#FFFFFF',
        depthTextColor: '#FFFFFF',
        currentDepthColor: '#FFFFFF',
        confidenceColor: '#CCCCCC',
        textBackground: 'rgba(0, 0, 0, 0.9)',
        depthArrowColor: '#FFFFFF',
      },
      ping360Settings: {
        lineColor: '#FFFFFF',
        radiusLineColor: '#CCCCCC',
        markerColor: '#FFFFFF',
      },
    },
  },
  highContrast: {
    description: 'High contrast mode for better visibility',
    settings: {
      commonSettings: {
        colorPalette: 'Monochrome White',
      },
      ping1DSettings: {
        depthLineColor: '#FFFFFF',
        depthTextColor: '#FFFFFF',
        currentDepthColor: '#FFFFFF',
        confidenceColor: '#FFFFFF',
        textBackground: 'rgba(0, 0, 0, 1)',
        depthArrowColor: '#FFFFFF',
      },
      ping360Settings: {
        lineColor: '#FFFFFF',
        radiusLineColor: '#FFFFFF',
        markerColor: '#FFFFFF',
        lineWidth: 1.0,
        radiusLineWidth: 1.0,
      },
    },
  },
};

const getPresetDescription = (preset) => {
  return presetConfigs[preset]?.description || '';
};

const handleReset = () => {
  selectedPreset.value = 'default';
  emit('reset');
  localStorage.removeItem('selectedAccessibilityPreset');
};

const handlePresetChange = (preset) => {
  if (preset === 'default') {
    handleReset();
    return;
  }

  const config = presetConfigs[preset].settings;

  Object.assign(localCommonSettings, {
    ...localCommonSettings,
    ...config.commonSettings,
  });

  Object.assign(localPing1DSettings, {
    ...localPing1DSettings,
    ...config.ping1DSettings,
  });

  Object.assign(localPing360Settings, {
    ...localPing360Settings,
    ...config.ping360Settings,
  });

  emit('update:commonSettings', { ...localCommonSettings });
  emit('update:ping1DSettings', { ...localPing1DSettings });
  emit('update:ping360Settings', { ...localPing360Settings });

  localStorage.setItem('selectedAccessibilityPreset', preset);
};

const updateColorPalette = (newPalette) => {
  localCommonSettings.colorPalette = newPalette;
  if (newPalette === 'Custom') {
    localCommonSettings.customPalette = colorPalettes.Custom;
  }
  emit('update:commonSettings', { ...localCommonSettings });
};

const saveSettings = () => {
  localStorage.setItem('common-settings', JSON.stringify(localCommonSettings));
  localStorage.setItem('ping1d-settings', JSON.stringify(localPing1DSettings));
  localStorage.setItem('ping360-settings', JSON.stringify(localPing360Settings));

  if (localCommonSettings.customPalette?.length > 0) {
    localStorage.setItem('customColorPalette', JSON.stringify(localCommonSettings.customPalette));
  }

  emit('save');
};

watch(
  () => props.commonSettings,
  (newSettings) => {
    Object.assign(localCommonSettings, newSettings);
  },
  { deep: true }
);

watch(
  () => props.ping1DSettings,
  (newSettings) => {
    Object.assign(localPing1DSettings, newSettings);
  },
  { deep: true }
);

watch(
  () => props.ping360Settings,
  (newSettings) => {
    Object.assign(localPing360Settings, newSettings);
  },
  { deep: true }
);

watch(
  localCommonSettings,
  (newSettings) => {
    emit('update:commonSettings', { ...newSettings });
  },
  { deep: true }
);

watch(
  localPing1DSettings,
  (newSettings) => {
    emit('update:ping1DSettings', { ...newSettings });
  },
  { deep: true }
);

watch(
  localPing360Settings,
  (newSettings) => {
    emit('update:ping360Settings', { ...newSettings });
  },
  { deep: true }
);
</script>