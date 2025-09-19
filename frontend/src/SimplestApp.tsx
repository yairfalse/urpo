function SimplestApp() {
  return (
    <div style={{
      padding: '20px',
      fontFamily: 'monospace',
      background: 'black',
      color: 'white',
      minHeight: '100vh',
      fontSize: '24px'
    }}>
      <h1 style={{ color: 'lime' }}>✅ REACT IS WORKING!</h1>
      <p>Timestamp: {new Date().toISOString()}</p>
      <button
        style={{
          padding: '10px 20px',
          fontSize: '18px',
          background: 'blue',
          color: 'white',
          border: 'none',
          cursor: 'pointer'
        }}
        onClick={() => alert('Button clicked!')}
      >
        Click Me
      </button>
    </div>
  );
}

export default SimplestApp;