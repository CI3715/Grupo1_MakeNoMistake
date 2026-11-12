'use client';

import React, { useEffect, useState } from 'react';
import { CampoTexto } from '../sesion/CampoTexto';
import type { ContextoSocial, ErroresCampos, TipoFallo } from '../../types/social';
import { erroresDeAccion } from '../../utils/erroresSocial';
import { crearGrupoCommand, type GrupoResumenDto } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useMontado } from '../../utils/useCargaRemota';
import { IconoMas } from '../Iconos';
import { BotonAccion } from './BotonAccion';
import { MensajeGeneral } from './MensajeGeneral';
import { Tarjeta } from './Tarjeta';

type CampoGrupo = 'nombre' | 'descripcion' | 'moneda' | 'actividad';
const CAMPOS: readonly CampoGrupo[] = ['nombre', 'descripcion', 'moneda', 'actividad'];
const ID_CAMPO: Record<CampoGrupo, string> = {
  nombre: 'grupo-nombre',
  descripcion: 'grupo-descripcion',
  moneda: 'grupo-moneda',
  actividad: 'grupo-actividad',
};

interface Props {
  ctx: ContextoSocial;
  /** Se llama con el grupo creado (para abrirlo o actualizar la lista). */
  onCreado: (grupo: GrupoResumenDto) => void;
}

/** Formulario para crear un grupo. Quien lo crea queda como administrador (RF-COM-01). */
export function FormularioCrearGrupo({ ctx, onCreado }: Props) {
  const [nombre, setNombre] = useState('');
  const [descripcion, setDescripcion] = useState('');
  const [moneda, setMoneda] = useState('USD');
  const [actividad, setActividad] = useState('');
  const [cargando, setCargando] = useState(false);
  const [errores, setErrores] = useState<ErroresCampos<CampoGrupo>>({});
  const [tipoError, setTipoError] = useState<TipoFallo>('otro');
  const montado = useMontado();
  const tema = temaSocial(ctx.modoOscuro);

  // Tras un error, el foco va al primer campo que falló.
  useEffect(() => {
    const primero = CAMPOS.find((c) => errores[c]);
    if (primero) document.getElementById(ID_CAMPO[primero])?.focus();
  }, [errores]);

  const enviar = async (e: React.FormEvent) => {
    e.preventDefault();
    if (cargando) return;
    setCargando(true);
    setErrores({});
    try {
      const grupo = await crearGrupoCommand({
        nombre,
        descripcion,
        moneda: moneda.trim().toUpperCase(),
        actividad,
      });
      if (!montado.current) return;
      setNombre('');
      setDescripcion('');
      setActividad('');
      onCreado(grupo);
    } catch (err) {
      if (montado.current) {
        const r = erroresDeAccion(err, CAMPOS);
        setErrores(r.errores);
        setTipoError(r.tipo);
      }
    } finally {
      if (montado.current) setCargando(false);
    }
  };

  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo="Crear grupo"
      descripcion="Un espacio para una actividad o un gasto recurrente."
    >
      <form onSubmit={enviar} noValidate className="space-y-3">
        <CampoTexto
          id={ID_CAMPO.nombre}
          etiqueta="Nombre"
          placeholder="Viaje a Mérida"
          autoComplete="off"
          valor={nombre}
          onCambio={setNombre}
          error={errores.nombre}
          deshabilitado={cargando}
          modoOscuro={ctx.modoOscuro}
        />
        <CampoTexto
          id={ID_CAMPO.descripcion}
          etiqueta="Descripción (opcional)"
          placeholder="Alojamiento, transporte y comidas"
          autoComplete="off"
          valor={descripcion}
          onCambio={setDescripcion}
          error={errores.descripcion}
          deshabilitado={cargando}
          modoOscuro={ctx.modoOscuro}
        />
        <CampoTexto
          id={ID_CAMPO.moneda}
          etiqueta="Moneda del grupo"
          placeholder="USD"
          autoComplete="off"
          maxLength={3}
          ayuda="Código de 3 letras, por ejemplo USD o EUR."
          valor={moneda}
          onCambio={setMoneda}
          error={errores.moneda}
          deshabilitado={cargando}
          modoOscuro={ctx.modoOscuro}
        />
        <CampoTexto
          id={ID_CAMPO.actividad}
          etiqueta="Fecha o actividad"
          placeholder="Viaje a Mérida, marzo"
          autoComplete="off"
          valor={actividad}
          onCambio={setActividad}
          error={errores.actividad}
          deshabilitado={cargando}
          modoOscuro={ctx.modoOscuro}
        />

        {errores.general && <MensajeGeneral ctx={ctx} mensaje={errores.general} tipo={tipoError} />}

        <div className={`rounded-xl border p-3 text-xs ${tema.avisoMorado}`}>
          <p className="font-semibold">Serás la persona administradora.</p>
          <p className="mt-0.5 opacity-90">
            Invita a tus amigos después de crear el grupo. Cada uno deberá aceptar.
          </p>
        </div>

        <BotonAccion modoOscuro={ctx.modoOscuro} tipo="submit" cargando={cargando} ancho>
          <span className="flex items-center justify-center gap-1.5">
            {!cargando && <IconoMas className="w-3.5 h-3.5" />}
            <span>{cargando ? 'Creando…' : 'Crear grupo'}</span>
          </span>
        </BotonAccion>
      </form>
    </Tarjeta>
  );
}
