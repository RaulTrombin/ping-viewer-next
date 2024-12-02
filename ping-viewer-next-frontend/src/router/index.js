import { setupLayouts } from 'virtual:generated-layouts';
import { createRouter, createWebHistory } from 'vue-router';
import { routes as autoRoutes } from 'vue-router/auto-routes';
import { widgetRoute } from './widget';

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    // Widget route needs to come first to take precedence
    widgetRoute,
    // Then the auto-generated routes
    ...setupLayouts(autoRoutes),
  ],
});

// Add this for debugging
router.beforeEach((to, from, next) => {
  next();
});

export default router;
