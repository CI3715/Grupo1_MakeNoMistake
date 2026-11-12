import React from 'react';

interface Props {
  onCrearCuenta: () => void;
}

export const EstadoVacio: React.FC<Props> = ({ onCrearCuenta }) => {
  return (
    <div className="flex flex-col items-center justify-center p-8 border-2 border-dashed border-gray-300 rounded-lg text-center my-6">
      <h3 className="text-lg font-semibold text-gray-700 mb-2">No tienes cuentas registradas</h3>
      <p className="text-sm text-gray-500 mb-4">
        Comienza creando tu primera cuenta para llevar el control de tus finanzas.
      </p>
      <button
        onClick={onCrearCuenta}
        className="px-4 py-2 bg-blue-600 text-white font-medium rounded-md hover:bg-blue-700 transition"
      >
        Crear la primera cuenta
      </button>
    </div>
  );
};