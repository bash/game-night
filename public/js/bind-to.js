for (const target of document.querySelectorAll('[data-bind-to]')) {
    const source = document.querySelector(`[name=${target.dataset.bindTo}]`)
    target.innerText = source.value
    source.addEventListener('input', () => target.innerText = source.value)
}
