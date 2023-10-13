export function bindDisabledTo(scope) {
    for (const target of scope.querySelectorAll('[data-bind-disabled-to]')) {
        const source = document.querySelector(`#${target.dataset.bindDisabledTo}`)
        target.disabled = !source.checked
        source.addEventListener('input', () => target.disabled = !source.checked)
    }
}

bindDisabledTo(document)
