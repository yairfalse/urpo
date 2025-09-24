import React from 'react';
import ReactDOM from 'react-dom/client';

console.log('main.tsx: Starting imports...');

import App from './App';
// import App from './AppPro';
// import App from './TestApp';
// import App from './AppDebug';
// import App from './SimplestApp';
// import App from './ProgressiveApp';
import './index.css';
import './styles/professional.css';
import './styles/sharp.css';

console.log('main.tsx: Imports complete, App:', App);

// BLAZING FAST: Measure startup performance
const startTime = performance.now();

// Initialize Tauri in development mode
const initializeTauri = async () => {
  if (typeof window !== 'undefined' && window.__TAURI__) {
    try {
      // Ensure Tauri APIs are available
      console.log('🚀 Tauri environment detected');

      // Test basic Tauri functionality
      const { invoke } = await import('@tauri-apps/api/tauri');
      console.log('✅ Tauri APIs loaded successfully');

      // Optional: Log window info for debugging
      const { appWindow } = await import('@tauri-apps/api/window');
      console.log('📱 Tauri window initialized:', appWindow.label);

    } catch (error) {
      console.error('❌ Tauri initialization error:', error);
    }
  } else {
    console.log('🌐 Running in web mode (no Tauri)');
  }
};

// Initialize and render app
const renderApp = async () => {
  try {
    console.log('renderApp: Starting...');

    // Initialize Tauri first
    await initializeTauri();

    // Ensure root element exists
    const rootElement = document.getElementById('root');
    console.log('renderApp: Root element:', rootElement);

    if (!rootElement) {
      throw new Error('Root element not found - check index.html');
    }

    console.log('renderApp: Creating React root...');

    // Render React app
    ReactDOM.createRoot(rootElement).render(
      <React.StrictMode>
        <App />
      </React.StrictMode>
    );

    console.log('renderApp: React render called');

    // Log startup time - target <200ms
    const endTime = performance.now();
    console.log(`React startup time: ${(endTime - startTime).toFixed(2)}ms`);

    if (endTime - startTime > 200) {
      console.warn('⚠️ Startup time exceeded 200ms target!');
    } else {
      console.log('✅ Startup time meets <200ms target');
    }
  } catch (error) {
    console.error('💥 Critical startup error:', error);

    // Fallback: show error message directly in DOM
    const rootElement = document.getElementById('root') || document.body;
    rootElement.innerHTML = `
      <div style="
        font-family: 'Monaco', 'Menlo', monospace;
        padding: 40px;
        background: #1a1a1a;
        color: #ff6b6b;
        height: 100vh;
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: center;
      ">
        <h1 style="margin: 0 0 20px 0; color: #ff6b6b;">🚨 URPO Startup Failed</h1>
        <p style="margin: 0 0 20px 0; color: #888;">Critical error during application initialization</p>
        <pre style="
          background: #2a2a2a;
          padding: 20px;
          border-radius: 8px;
          color: #ffd93d;
          font-size: 12px;
          overflow: auto;
          max-width: 80%;
        ">${error instanceof Error ? error.message : String(error)}</pre>
        <button onclick="window.location.reload()" style="
          margin-top: 20px;
          padding: 10px 20px;
          background: #007acc;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
        ">🔄 Reload Application</button>
      </div>
    `;
  }
};

// Start the app
renderApp().catch(console.error);