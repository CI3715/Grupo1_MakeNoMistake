'use client';

import React, { useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { PantallaCuentas } from './components/PantallaCuentas';
import { PantallaTasas } from './components/PantallaTasas';
import { PantallaPatrimonio } from './components/PantallaPatrimonio';

export type VistaActual = 'mis-cuentas' | 'detalle-cuenta' | 'tasas' | 'patrimonio';

export default function Home() {
  const [modoOscuro, setModoOscuro] = useState(true);
  const [vistaActual, setVistaActual] = useState<VistaActual>('mis-cuentas');

  const bgMain = modoOscuro ? 'bg-[#0B0F19] text-white' : 'bg-[#F6F5FA] text-slate-900';

  return (
    <div className={`flex min-h-screen w-full font-sans transition-colors duration-200 ${bgMain}`}>
      {/* MENÚ LATERAL */}
      <Sidebar
        modoOscuro={modoOscuro}
        toggleModoOscuro={() => setModoOscuro(!modoOscuro)}
        pantallaActual={vistaActual}
        setPantallaActual={(pantalla) => setVistaActual(pantalla as VistaActual)}
      />

      {/* ÁREA DE CONTENIDO PRINCIPAL */}
      <main className="flex-1 p-8 overflow-y-auto">
        {(vistaActual === 'mis-cuentas' || vistaActual === 'detalle-cuenta') && (
          <PantallaCuentas
            modoOscuro={modoOscuro}
            vistaSubpantalla={vistaActual}
            setVistaSubpantalla={setVistaActual}
          />
        )}

        {vistaActual === 'tasas' && <PantallaTasas modoOscuro={modoOscuro} />}

        {vistaActual === 'patrimonio' && <PantallaPatrimonio modoOscuro={modoOscuro} />}
      </main>
    </div>
  );
}