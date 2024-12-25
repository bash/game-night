for (const target of document.querySelectorAll('[data-open-modal]')) {
    target.addEventListener('click', () => {
      const dialog = document.querySelector(`#${target.dataset.openModal}`)
      dialog.showModal()
    })
}
