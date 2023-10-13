export function validateGte(scope) {
    const validate = (left, right) => left.setCustomValidity(
        left.valueAsNumber >= right.valueAsNumber
            ? ''
            : `Please select a value that is no less than ${right.value}`)

    for (const left of scope.querySelectorAll('[data-validate-gte]')) {
        const right = document.getElementById(left.dataset.validateGte)
        left.addEventListener('input', () => validate(left, right))
        right.addEventListener('input', () => validate(left, right))
        validate(left, right)
    }
}

validateGte(document)
