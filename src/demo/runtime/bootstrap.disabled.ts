import type { ComponentType } from 'react';

export function getRootComponent(productionApp: ComponentType): ComponentType {
  return productionApp;
}
