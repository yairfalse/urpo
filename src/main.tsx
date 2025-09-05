import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';

// BLAZING FAST: Measure startup performance
const startTime = performance.now();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// Log startup time - target <200ms
const endTime = performance.now();
console.log(`React startup time: ${(endTime - startTime).toFixed(2)}ms`);

if (endTime - startTime > 200) {
  console.warn('⚠️ Startup time exceeded 200ms target!');
}