import { bindDisabledTo } from '/js/bind-disabled-to.js'
import { validateGte } from '/js/validate-gte.js'

for (const button of document.querySelectorAll('[data-fetch-more-target]')) {
    const target = document.getElementById(button.dataset.fetchMoreTarget)
    const url = button.dataset.fetchMoreUrl
    button.addEventListener('click', fetchAndUpdateCalendar(button, url, target))
}

function fetchAndUpdateCalendar(button, url, target) {
    const state = { count: 1, fetching: false }
    return async () => {
        button.style.cursor = 'wait'
        const html = await fetchCalendar(state, url, button.closest('form'))
        button.style.cursor = ''
        target.innerHTML = html
        bindDisabledTo(target)
        validateGte(target)
    }
}

async function fetchCalendar(state, url, formElement) {
    if (state.fetching) return
    state.fetching = true
    state.count += 1

    try {
        const body = new FormData(formElement)
        body.append('count', state.count)
        const response = await fetch(url, { method: 'post', credentials: 'same-origin', body })
        return await response.text()
    }
    finally {
        state.fetching = false
    }
}
