// Claim all clients as soon as possible
self.addEventListener('install', _event => self.skipWaiting())
self.addEventListener('activate', _event => self.clients.claim())

self.addEventListener('push', event => {
    if (!(self.Notification && self.Notification.permission === "granted")) {
        return
    }

    // A superset of what the showNotification() options accept
    // https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerRegistration/showNotification#options
    const data = event.data?.json() ?? {}
    const { title } = data

    if (title == '' || title == null) {
        console.warn('Dropping notification without title', data)
        return
    }

    event.waitUntil(self.registration.showNotification(title, { ...data, data }))
})

self.addEventListener('notificationclick', event => {
    const { data } = event.notification
    const actions = data.actions ?? []
    const onClick = event.action == '' ? data.onClick : actions.find(a => a.action == event.action)?.onClick
    event.waitUntil(executeAction(onClick))
})

async function executeAction(onClick) {
    if (onClick == null) return

    switch (onClick.action) {
        case 'openRelative':
            await openUrl(new URL(onClick.url, self.location))
            break
        default:
            console.warn(`Unrecognized onClick action: ${onClick.action}`, onClick)
            break
    }
}

async function openUrl(url) {
    const windowClients = await self.clients.matchAll({ includeUncontrolled: true, type: 'window' })
    const clientWithDesiredUrl = windowClients.find(client => isEqualIgnoreHash(client.url, url))

    if (clientWithDesiredUrl != null) {
        await clientWithDesiredUrl.focus()
    } else if (windowClients.length >= 1) {
        await windowClients[0].navigate(url)
    } else {
        await self.clients.openWindow(url)
    }
}

function isEqualIgnoreHash(left, right) {
    const leftUrl = new URL(left)
    leftUrl.hash = ''
    const rightUrl = new URL(right)
    rightUrl.hash = ''
    return leftUrl.href === rightUrl.href
}
