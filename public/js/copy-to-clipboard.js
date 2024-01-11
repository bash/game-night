for (const target of document.querySelectorAll('[data-copy-to-clipboard]')) {
    const labelElement = target.querySelector('span')
    const label = labelElement.innerText
    let resetLabelTimer

    target.disabled = (navigator.clipboard == undefined || navigator.clipboard.writeText == undefined)

    target.addEventListener('click', async () => {
        await navigator.clipboard.writeText(target.dataset.copyToClipboard)
        clearTimeout(resetLabelTimer)
        labelElement.innerText = "Copied!"
        resetLabelTimer = setTimeout(() => labelElement.innerText = label, 1_000)
    })
}
