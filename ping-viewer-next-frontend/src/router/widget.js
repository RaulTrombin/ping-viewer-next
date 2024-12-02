import WidgetContent from '../pages/addons/widget/[type]/WidgetContent.vue';

export const widgetRoute = {
  path: '/addons/widget/:type',
  name: 'SonarWidget',
  component: WidgetContent,
  // This ensures we don't use the default layout
  meta: {
    layout: 'empty',
  },
};
