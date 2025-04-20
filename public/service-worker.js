// Claim all clients as soon as possible
self.addEventListener('install', _event => self.skipWaiting())
self.addEventListener('activate', _event => self.clients.claim())

self.addEventListener('push', logErrors(event => {
    if (!(self.Notification && self.Notification.permission === "granted")) {
        return
    }

    // A superset of what the showNotification() options accept
    // https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerRegistration/showNotification#options
    const data = event.data?.json() ?? {}
    const { web_push, notification } = data

    if (web_push != 8030 || notification == null) {
      console.warn('Dropping push with invalid structure', data)
      return
    }

    const { title } = notification;
    if (title == '' || title == null) {
        console.warn('Dropping notification without title', data)
        return
    }

    console.log(title, notification)
    event.waitUntil(self.registration.showNotification(title, { ...notification, data: notification }))
}))

self.addEventListener('notificationclick', logErrors(event => {
    const { data } = event.notification
    const actions = data.actions ?? []
    const navigation = hasAction(event) ? actions.find(a => a.action == event.action)?.navigate : data.navigate
    event.notification.close()
    event.waitUntil(navigate(navigation))
}))

async function navigate(navigation) {
    if (navigation == null) return
    const url = new URL(navigation, self.location)
    const windowClients = await self.clients.matchAll({ includeUncontrolled: true, type: 'window' })
    const clientWithDesiredUrl = windowClients.find(client => isEqualIgnoreHash(client.url, url))

    if (clientWithDesiredUrl != null) {
        await clientWithDesiredUrl.focus()
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

function logErrors(f) {
  return (...args) => {
    try {
      f(...args)
    } catch (e) {
      console.error(e)
      throw e
    }
  }
}

function hasAction(event) {
  return event.action !== undefined && event.action !== ''
}
