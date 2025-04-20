navigator.serviceWorker.register('/service-worker.js', { type: 'module', scope: '/' })
    .then(registration => registration.update())
    .catch(error => console.warn('Service worker registration failed', error))
