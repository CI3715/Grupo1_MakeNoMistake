/**
 * Lleva el foco al primer campo con error (aria-invalid) del formulario. Se llama justo
 * después de un envío fallido, cuando React ya pintó los errores debajo de cada campo.
 */
export function enfocarPrimerError(): void {
  window.setTimeout(() => {
    const campo = document.querySelector<HTMLElement>('[aria-invalid="true"]');
    campo?.focus();
  }, 0);
}
