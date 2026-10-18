import React from 'react';
import { IconoBilletera, IconoMas } from './Iconos';

interface Props {
  onCrearCuenta: () => void;
}

export const EstadoVacio: React.FC<Props> = ({ onCrearCuenta }) => {
  return (
    <div className="flex flex-col items-center px-6 py-16 text-center">
      <span className="mb-4 grid size-14 place-items-center rounded-2xl bg-acento-suave text-acento-texto">
        <IconoBilletera width={26} height={26} />
      </span>
      <h3 className="text-lg font-semibold">No tienes cuentas registradas</h3>
      <p className="mt-1 max-w-sm text-sm text-suave">
        Comienza creando tu primera cuenta para llevar el control de tus finanzas.
      </p>
      <button
        onClick={onCrearCuenta}
        className="mt-6 inline-flex h-10 items-center gap-2 rounded-xl bg-acento px-5 text-sm font-medium text-acento-fg transition hover:brightness-110 active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-acento"
      >
        <IconoMas width={16} height={16} />
        Crear la primera cuenta
      </button>
    </div>
  );
};
