'use client';

import React from 'react';
import type { CampoGrupo, ValoresGrupo } from '../../types/social';
import type { ErroresSesion } from '../../types/sesion';
import { temaSocial } from '../../utils/temaSocial';
import { CampoTexto } from '../sesion/CampoTexto';
import { BotonSocial, TarjetaSocial } from './ElementosSociales';

interface Props {
  modoOscuro: boolean;
  valores: ValoresGrupo;
  onCambio: (campo: CampoGrupo, valor: string) => void;
  errores: ErroresSesion<CampoGrupo>;
  cargando: boolean;
  onEnviar: () => void;
}

/** Crear un grupo. El error de cada campo sale debajo de él. */
export const FormularioGrupo: React.FC<Props> = ({ modoOscuro, valores, onCambio, errores, cargando, onEnviar }) => {
  const tema = temaSocial(modoOscuro);
  return (
    <TarjetaSocial
      modoOscuro={modoOscuro}
      titulo="Crear grupo"
      descripcion="Un espacio para una actividad o un gasto recurrente."
    >
      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          onEnviar();
        }}
        noValidate
      >
        <CampoTexto
          id="grupo-nombre"
          etiqueta="Nombre"
          valor={valores.nombre}
          onCambio={(v) => onCambio('nombre', v)}
          placeholder="Viaje de diciembre"
          error={errores.nombre}
          autoComplete="off"
          deshabilitado={cargando}
          modoOscuro={modoOscuro}
        />
        <CampoTexto
          id="grupo-descripcion"
          etiqueta="Descripción (opcional)"
          valor={valores.descripcion}
          onCambio={(v) => onCambio('descripcion', v)}
          placeholder="Alojamiento, transporte y comidas"
          error={errores.descripcion}
          autoComplete="off"
          deshabilitado={cargando}
          modoOscuro={modoOscuro}
        />
        <CampoTexto
          id="grupo-moneda"
          etiqueta="Moneda del grupo"
          valor={valores.moneda}
          onCambio={(v) => onCambio('moneda', v.toUpperCase())}
          placeholder="USD"
          ayuda="Código de 3 letras, por ejemplo USD, EUR o VES."
          error={errores.moneda}
          autoComplete="off"
          maxLength={3}
          lista="monedas-grupo"
          deshabilitado={cargando}
          modoOscuro={modoOscuro}
        />
        <datalist id="monedas-grupo">
          <option value="USD" />
          <option value="EUR" />
          <option value="VES" />
        </datalist>
        <CampoTexto
          id="grupo-actividad"
          etiqueta="Fecha o actividad"
          valor={valores.actividad}
          onCambio={(v) => onCambio('actividad', v)}
          placeholder="Viaje a Mérida, marzo"
          error={errores.actividad}
          autoComplete="off"
          deshabilitado={cargando}
          modoOscuro={modoOscuro}
        />
        <div className={`rounded-xl border px-3 py-2.5 text-[11px] ${tema.avisoMorado}`}>
          <p className="font-semibold">Serás el administrador.</p>
          <p className="mt-0.5 opacity-90">
            Invita a tus amigos después de crear el grupo. Cada uno deberá aceptar para entrar.
          </p>
        </div>
        <BotonSocial modoOscuro={modoOscuro} type="submit" cargando={cargando} ancho>
          {cargando ? 'Creando grupo…' : 'Crear grupo'}
        </BotonSocial>
      </form>
    </TarjetaSocial>
  );
};
