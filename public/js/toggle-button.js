for (const button of document.querySelectorAll('.toggle-button')) {
    const label = button.querySelector(':scope > label');
    const input = document.getElementById(label.htmlFor)

    label.addEventListener('keydown', event => {
        if (event.key == ' ' || event.key == 'Enter') event.preventDefault()
    })

    label.addEventListener('keyup', (event) => {
        switch (event.key) {
            case ' ':
                event.preventDefault()
                input.checked = !input.checked
                break
            case 'Enter':
                event.preventDefault()
                input.form.submit()
                break
        }
    })
}
