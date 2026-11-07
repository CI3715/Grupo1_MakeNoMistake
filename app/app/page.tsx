'use client';

import React, { useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { PantallaCuentas } from './components/PantallaCuentas';
import { PantallaTasas } from './components/PantallaTasas';
import { PantallaPatrimonio } from './components/PantallaPatrimonio';
import { PantallaSesion } from './components/sesion/PantallaSesion';
import { BienvenidaPrimerAcceso } from './components/sesion/BienvenidaPrimerAcceso';

export default function Home() {
  const [pantallaActual, setPantallaActual] = useState<string>('mis-cuentas');
  const [modoOscuro, setModoOscuro] = useState(true);

  const toggleModoOscuro = () => {
    setModoOscuro((prev) => !prev);
  };

  // Mapeamos el estado principal al tipo exacto esperado por PantallaCuentas
  const vistaSubpantalla: 'mis-cuentas' | 'detalle-cuenta' =
    pantallaActual === 'detalle-cuenta' ? 'detalle-cuenta' : 'mis-cuentas';

  const handleCambiarSubpantalla = (
    vista: 'mis-cuentas' | 'detalle-cuenta' | 'tasas' | 'patrimonio'
  ) => {
    setPantallaActual(vista);
  };

  return (
    <div
      className={`flex h-screen overflow-hidden ${
        modoOscuro ? 'bg-[#0B0F17] text-white' : 'bg-slate-50 text-slate-900'
      }`}
    >
      <Sidebar
        modoOscuro={modoOscuro}
        toggleModoOscuro={toggleModoOscuro}
        pantallaActual={pantallaActual}
        setPantallaActual={setPantallaActual}
      />
      <main className="flex-1 overflow-y-auto p-6 min-w-0">
        <div className="max-w-[752px] mx-auto min-w-[320px]">
          {(pantallaActual === 'mis-cuentas' || pantallaActual === 'detalle-cuenta') && (
            <PantallaCuentas
              modoOscuro={modoOscuro}
              vistaSubpantalla={vistaSubpantalla}
              setVistaSubpantalla={handleCambiarSubpantalla}
            />
          )}
          {pantallaActual === 'tasas' && <PantallaTasas modoOscuro={modoOscuro} />}
          {pantallaActual === 'patrimonio' && (
            <PantallaPatrimonio modoOscuro={modoOscuro} />
          )}
          {(pantallaActual === 'inicio-sesion' || pantallaActual === 'registro') && (
            <PantallaSesion
              modoOscuro={modoOscuro}
              vista={pantallaActual}
              setPantallaActual={setPantallaActual}
            />
          )}
        </div>
      </main>
      <BienvenidaPrimerAcceso modoOscuro={modoOscuro} setPantallaActual={setPantallaActual} />
    </div>
  );
}