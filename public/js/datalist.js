for (const datalist of document.querySelectorAll('[data-datalist-href]')) {
    let materialized = false

    for (const reference of document.querySelectorAll(`[list='${datalist.id}']`)) {
        reference.addEventListener('focus', async _ => {
            if (materialized) return
            materialized = true

            const response = await fetch(datalist.dataset.datalistHref)
            const options = await response.json()

            for (const option of options) {
                const optionElement = document.createElement('option')
                optionElement.value = option
                datalist.append(optionElement)
            }
        })
    }
}
