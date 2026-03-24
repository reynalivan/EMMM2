## Dashboard and Onboarding Visual Upgrade

### Context
Modernized the dashboard statistics and restored the onboarding success checkmark to provide a more premium and theme-aware user experience.

### Changes
- **Dashboard Charts**: Replaced hardcoded black colors with DaisyUI semantic CSS variables. Added SVG gradients for depth and backdrop blurs for glassmorphism.
- **Onboarding Success**: Restored the missing checkmark in `AutoDetectResult`. Enhanced it with a spring animation, success glow, and vibrant coloring.
- **Global Mocking**: Added a global mock for `motion/react` in `setupTests.ts` to stabilize the test environment for animated components.
- **Test Alignment**: Updated `AutoDetectResult.test.tsx` and `Dashboard.test.tsx` to match the new UI structures and handle i18n keys correctly.

### Impacted Files
- `src/features/dashboard/Dashboard.tsx` (modified)
- `src/features/onboarding/AutoDetectResult.tsx` (modified)
- `src/setupTests.ts` (modified)
- `src/features/dashboard/Dashboard.test.tsx` (modified)
- `src/features/onboarding/AutoDetectResult.test.tsx` (modified)

### Goal
A vibrant, theme-aware dashboard and a recognizable success state in the onboarding flow.

### Impact
- Improved visual engagement and perceived quality.
- Consistent look-and-feel across Onyx and Light themes.
- Better test stability for components using Framer Motion.
