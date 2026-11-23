'use client';

import React, { Fragment, useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { PantallaCuentas } from './components/PantallaCuentas';
import { PantallaTasas } from './components/PantallaTasas';
import { PantallaPatrimonio } from './components/PantallaPatrimonio';
import { PantallaSesion } from './components/sesion/PantallaSesion';
import { PantallaSocial } from './components/social/PantallaSocial';
import { BienvenidaPrimerAcceso } from './components/sesion/BienvenidaPrimerAcceso';
import { AvisoSesionVencida } from './components/sesion/AvisoSesionVencida';
import { useSesion } from './utils/useSesion';
import { useSincronizacion } from './utils/useSincronizacion';

export default function Home() {
  const [pantallaActual, setPantallaActual] = useState<string>('mis-cuentas');
  const [modoOscuro, setModoOscuro] = useState(true);
  const control = useSesion();
  const sincronizacion = useSincronizacion({
    sesionEstado: control.sesion.estado,
    claveLibro: control.claveLibro,
  });

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
        control={control}
      />
      <main className="relative flex-1 overflow-y-auto p-6 min-w-0">
        <div className="max-w-[752px] mx-auto min-w-[320px]">
          {control.avisoVencidaVisible && (
            <AvisoSesionVencida
              modoOscuro={modoOscuro}
              mostrarBoton={pantallaActual !== 'inicio-sesion'}
              onIniciarSesion={() => setPantallaActual('inicio-sesion')}
              onCerrar={control.ocultarAviso}
            />
          )}
          {/* Al iniciar o cerrar sesión cambia la clave: las pantallas se montan de nuevo y piden
              sus datos al libro nuevo, sin dejar a la vista nada del usuario anterior. Las pantallas
              de sesión quedan fuera para no desmontar el formulario que está iniciando sesión. */}
          <Fragment key={control.claveLibro}>
            {(pantallaActual === 'mis-cuentas' || pantallaActual === 'detalle-cuenta') && (
              <PantallaCuentas
                modoOscuro={modoOscuro}
                vistaSubpantalla={vistaSubpantalla}
                setVistaSubpantalla={handleCambiarSubpantalla}
                versionDatos={sincronizacion.versionDatos}
              />
            )}
            {pantallaActual === 'tasas' && <PantallaTasas modoOscuro={modoOscuro} />}
            {pantallaActual === 'patrimonio' && (
              <PantallaPatrimonio modoOscuro={modoOscuro} versionDatos={sincronizacion.versionDatos} />
            )}
            {(pantallaActual === 'amigos' || pantallaActual === 'grupos') && (
              <PantallaSocial
                modoOscuro={modoOscuro}
                vista={pantallaActual}
                setPantallaActual={setPantallaActual}
                sincronizacion={sincronizacion}
              />
            )}
          </Fragment>
          {(pantallaActual === 'inicio-sesion' ||
            pantallaActual === 'registro' ||
            pantallaActual === 'perfil') && (
            <PantallaSesion
              modoOscuro={modoOscuro}
              vista={pantallaActual}
              setPantallaActual={setPantallaActual}
              control={control}
            />
          )}
        </div>
      </main>
      <BienvenidaPrimerAcceso modoOscuro={modoOscuro} setPantallaActual={setPantallaActual} />
    </div>
  );
}