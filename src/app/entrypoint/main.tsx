import React from 'react';
import ReactDOM from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import '@/shared/i18n/config'; // Initialize i18n early
import { queryClient } from '@/shared/lib/queryClient';
import App from './App.tsx';
import { getRootComponent } from '@/demo/bootstrap';
import './App.css';
import { reportFrontendError } from '@/shared/lib/telemetry';

const RootComponent = getRootComponent(App);

// Disable native browser context menu so Radix context menus work in Tauri WebView
document.addEventListener('contextmenu', (e) => {
  // Allow native context menu only on text inputs/textareas for copy-paste
  const target = e.target as HTMLElement;
  if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return;
  e.preventDefault();
});

window.addEventListener('error', (event) => {
  reportFrontendError(event.error ?? event.message, 'window_error', { promptUser: true });
});

window.addEventListener('unhandledrejection', (event) => {
  reportFrontendError(event.reason, 'unhandled_rejection', { promptUser: true });
});

type RootWindow = Window & { __emmmReactRoot?: Root };

const rootContainer = document.getElementById('root') as HTMLElement;
const rootWindow = window as RootWindow;
const root =
  rootWindow.__emmmReactRoot ??
  ReactDOM.createRoot(rootContainer, {
    onUncaughtError: (error) => reportFrontendError(error, 'react_render', { promptUser: true }),
  });

rootWindow.__emmmReactRoot = root;

root.render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <RootComponent />
      </BrowserRouter>
    </QueryClientProvider>
  </React.StrictMode>,
);
