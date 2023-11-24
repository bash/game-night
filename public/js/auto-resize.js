for (const textarea of document.querySelectorAll('textarea[data-auto-resize]')) {
    textarea.style.resize = 'none'
    textarea.style.boxSizing = 'border-box'

    const { borderTopWidth, borderBottomWidth } = window.getComputedStyle(textarea)

    const resize = () => {
        textarea.style.height = 'auto'
        textarea.style.height = `calc(${textarea.scrollHeight}px + ${borderBottomWidth} + ${borderTopWidth})`
    }

    textarea.addEventListener('input', resize)
    resize()
}
