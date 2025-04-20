const State = {
    WAITING_FOR_SERVICE_WORKER: 1,
    NOT_SUBSCRIBED: 2,
    SUBSCRIBING: 3,
    SUBSCRIBED: 4,
    UNSUBSCRIBING: 5,
}
class PushSubscriptionButton extends HTMLElement {
    #state = State.WAITING_FOR_SERVICE_WORKER
    #endpoints
    #button
    #errorElement
    #pushManager
    #error

    connectedCallback() {
        this.#endpoints = JSON.parse(this.getAttribute('endpoints'))
        this.#button = document.createElement('button')
        this.#button.addEventListener('click', (event) => this.#onClick(event))
        this.#errorElement = document.createElement('span')
        this.#errorElement.classList.add('error-message', '-inline')
        this.append(this.#button, ' ', this.#errorElement)
        this.#update()
        this.#getPushManager()
    }

    #setState(state, error) {
        this.#error = error
        this.#state = state
        this.#update()
    }

    #update() {
        this.#updateEnabled()
        this.#updateLabel()
        this.#updateError()
    }

    #updateEnabled() {
        this.#button.disabled = !(
        this.#state === State.SUBSCRIBED ||
        this.#state === State.NOT_SUBSCRIBED)
    }

    #updateLabel() {
        switch (this.#state) {
        case State.WAITING_FOR_SERVICE_WORKER:
        case State.NOT_SUBSCRIBED:
            this.#button.innerText = 'Subscribe on this device'
            break
        case State.SUBSCRIBING:
            this.#button.innerText = 'Subscribing...'
            break
        case State.SUBSCRIBED:
            this.#button.innerText = 'Unsubscribe'
            break
        case State.UNSUBSCRIBING:
            this.#button.innerText = 'Unsubscribing...'
            break
        }
    }

    #updateError() {
        this.#errorElement.toggleAttribute('hidden', !this.#error)
        this.#errorElement.innerText = this.#error
    }

    async #getPushManager() {
        const { pushManager } = await navigator.serviceWorker.ready
        const subscription = await pushManager.getSubscription()
        const state = subscription ? State.SUBSCRIBED : State.NOT_SUBSCRIBED
        this.#pushManager = pushManager
        this.#setState(state)
    }

    async #onClick(event) {
        event.preventDefault()
        const oldState = this.#state
        try {
            switch (oldState) {
            case State.NOT_SUBSCRIBED:
                this.#setState(State.SUBSCRIBING)
                await this.#subscribe()
                this.#setState(State.SUBSCRIBED)
                break
            case State.SUBSCRIBED:
                this.#setState(State.UNSUBSCRIBING)
                await this.#unsubscribe()
                this.#setState(State.NOT_SUBSCRIBED)
            }
        } catch (e) {
            const errorMessage = Notification.permission === "denied"
                ? 'please allow this website to send notifications'
                : 'uh-oh something went wrong'
            console.error(e)
            this.#setState(oldState, errorMessage)
        }
    }

    async #subscribe() {
        const webPushKey = await fetch(this.#endpoints.get_public_key).then(r => r.text())
        const subscription = await this.#pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: webPushKey })
        const { endpoint, keys } = subscription.toJSON()
        await postJson(this.#endpoints.subscribe, { endpoint, keys })
    }

    async #unsubscribe() {
        const subscription = await this.#pushManager.getSubscription()
        if (subscription) {
            await subscription.unsubscribe()
            await postJson(this.#endpoints.unsubscribe, { endpoint: subscription.endpoint })
        }
    }
}

async function postJson(url, body) {
    const response = await fetch(url, {
        method: 'post',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body),
    })
    if (!response.ok) {
        throw new Error(`non-ok response with status ${response.status}: ${response.statusText}`)
    }
    return response
}

customElements.define('push-subscription-button', PushSubscriptionButton)
