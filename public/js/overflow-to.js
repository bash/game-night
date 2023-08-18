for (const source of document.querySelectorAll('[data-overflow-to]')) {
    const target = document.querySelector(`[name='${source.dataset.overflowTo}']`)
    overflow(source, target)
    source.addEventListener('input', () => overflow(source, target))
}

function overflow(source, target) {
    const value = source.value.trim()
    const firstSpace = value.indexOf(' ')
    if (firstSpace >= 0) {
        source.value = value.substring(0, firstSpace).trim()
        target.value = value.substring(firstSpace + 1).trim()
        target.focus()
        target.dispatchEvent(new CustomEvent('input'))
    }
}
