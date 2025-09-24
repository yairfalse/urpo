import React from 'react';

const TestSimple = () => {
  return (
    <div style={{
      background: '#0a0a0a',
      color: 'white',
      padding: '20px',
      minHeight: '100vh',
      fontFamily: 'monospace'
    }}>
      <h1>🚀 URPO Test Page</h1>
      <p>If you can see this, the frontend is working!</p>
      <div style={{
        background: '#1a1a1a',
        border: '1px solid #333',
        padding: '15px',
        borderRadius: '8px',
        marginTop: '20px'
      }}>
        <h2>✅ Frontend Status: Working</h2>
        <p>Sharp CSS loaded: Yes</p>
        <p>React rendering: Yes</p>
        <p>Styles applied: Yes</p>
      </div>
    </div>
  );
};

export default TestSimple;