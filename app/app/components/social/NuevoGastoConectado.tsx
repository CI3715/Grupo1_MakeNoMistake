'use client';

import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  previsualizarDivisionCommand,
  registrarGastoGrupoCommand,
  verGrupoCommand,
  type GrupoDetalleDto,
  type ParticipantePayload,
  type PrevisualizacionDivisionDto,
  type ReglaDivision,
} from '../../utils/tauriCommands';
import { fechaHoyLocal, normalizarDecimal } from '../../utils/formato';
import { erroresDeFormulario } from '../../utils/erroresSesion';
import { useCargaSocial } from '../../utils/useCargaSocial';
import { useEnvio } from '../../utils/useEnvio';
import { enfocarPrimerError } from '../../utils/enfocar';
import type { CampoGasto } from '../../types/social';
import { AvisoSocial, CargandoSocial, EncabezadoSocial } from './ElementosSociales';
import { FormularioGasto, type ValoresGasto } from './FormularioGasto';

const CAMPOS_GASTO: readonly CampoGasto[] = [
  'concepto',
  'monto',
  'fecha',
  'pagador',
  'regla',
  'participantes',
  'porcentajes',
  'montos',
];

/** Espera tras la última tecla antes de pedir la vista previa. */
const ESPERA_VISTA_PREVIA_MS = 250;

interface Props {
  modoOscuro: boolean;
  grupoId: string;
  onVolver: () => void;
  onRegistrado: () => void;
}

/** Carga el grupo (miembros y moneda) y monta el formulario cuando llegan los datos. */
export function NuevoGastoConectado({ modoOscuro, grupoId, onVolver, onRegistrado }: Props) {
  const cargar = useCallback(() => verGrupoCommand(grupoId), [grupoId]);
  const { estado, recargar } = useCargaSocial(cargar);
  const volver = { texto: 'Volver al grupo', onClick: onVolver };

  if (estado.tipo === 'cargando') {
    return (
      <div>
        <EncabezadoSocial modoOscuro={modoOscuro} titulo="Nuevo gasto compartido" volver={volver} />
        <CargandoSocial modoOscuro={modoOscuro} filas={2} />
      </div>
    );
  }
  if (estado.tipo === 'error') {
    return (
      <div>
        <EncabezadoSocial modoOscuro={modoOscuro} titulo="Nuevo gasto compartido" volver={volver} />
        <AvisoSocial modoOscuro={modoOscuro} tono="ambar" accion={{ texto: 'Reintentar', onClick: recargar }}>
          {estado.mensaje}
        </AvisoSocial>
      </div>
    );
  }
  return (
    <FormularioGastoConectado
      modoOscuro={modoOscuro}
      detalle={estado.datos}
      onVolver={onVolver}
      onRegistrado={onRegistrado}
    />
  );
}

interface VistaPrevia {
  /** Datos con los que se calculó: si ya no coinciden con lo escrito, no se muestra. */
  clave: string;
  resultado: PrevisualizacionDivisionDto | null;
  aviso: string | null;
}

function FormularioGastoConectado({
  modoOscuro,
  detalle,
  onVolver,
  onRegistrado,
}: {
  modoOscuro: boolean;
  detalle: GrupoDetalleDto;
  onVolver: () => void;
  onRegistrado: () => void;
}) {
  const { grupo, miembros } = detalle;
  const [valores, setValores] = useState<ValoresGasto>(() => ({
    concepto: '',
    monto: '',
    fecha: fechaHoyLocal(),
    pagador: miembros[0]?.usuario ?? '',
    regla: 'partes_iguales',
  }));
  const [excluidos, setExcluidos] = useState<string[]>([]);
  const [valoresParticipantes, setValoresParticipantes] = useState<Record<string, string>>({});
  const [previa, setPrevia] = useState<VistaPrevia | null>(null);
  const envio = useEnvio<CampoGasto>(CAMPOS_GASTO);

  const participantes = useMemo<ParticipantePayload[]>(
    () =>
      miembros
        .filter((m) => !excluidos.includes(m.usuario))
        .map((m) => ({
          usuario: m.usuario,
          valor: valores.regla === 'partes_iguales' ? '' : normalizarDecimal(valoresParticipantes[m.usuario] ?? ''),
        })),
    [miembros, excluidos, valores.regla, valoresParticipantes]
  );
  const monto = normalizarDecimal(valores.monto);
  const regla = valores.regla;
  const listo = monto !== '' && participantes.length > 0;
  const clave = JSON.stringify({ monto, regla, participantes });

  // Vista previa en vivo (cálculo local con decimales exactos, sin red). El estado solo se
  // actualiza dentro del callback de la promesa.
  useEffect(() => {
    if (!listo) return;
    let vigente = true;
    const espera = window.setTimeout(() => {
      previsualizarDivisionCommand({ monto, regla, participantes })
        .then((resultado) => {
          if (vigente) setPrevia({ clave, resultado, aviso: null });
        })
        .catch((err) => {
          if (vigente) {
            setPrevia({ clave, resultado: null, aviso: erroresDeFormulario(err, []).general ?? null });
          }
        });
    }, ESPERA_VISTA_PREVIA_MS);
    return () => {
      vigente = false;
      window.clearTimeout(espera);
    };
  }, [clave, listo, monto, regla, participantes]);

  const vigenteVista = listo && previa !== null && previa.clave === clave ? previa : null;

  const cambiar = (campo: keyof ValoresGasto, valor: string) => {
    setValores((v) => ({ ...v, [campo]: campo === 'regla' ? (valor as ReglaDivision) : valor }));
    // Un porcentaje no significa lo mismo que un monto: al cambiar de regla se empieza limpio.
    if (campo === 'regla') setValoresParticipantes({});
  };

  const registrar = async () => {
    const r = await envio.enviar(() =>
      registrarGastoGrupoCommand({
        grupo_id: grupo.id,
        concepto: valores.concepto.trim(),
        monto,
        fecha: valores.fecha,
        pagador: valores.pagador,
        regla,
        participantes,
      })
    );
    if (r.ok) onRegistrado();
    else enfocarPrimerError();
  };

  return (
    <div>
      <EncabezadoSocial
        modoOscuro={modoOscuro}
        titulo="Nuevo gasto compartido"
        subtitulo={`${grupo.nombre} / Nuevo gasto · Borrador, aún no cambia los balances.`}
        volver={{ texto: 'Volver al grupo', onClick: onVolver }}
        enfocar
      />
      <FormularioGasto
        modoOscuro={modoOscuro}
        grupo={grupo}
        miembros={miembros}
        valores={valores}
        onCambio={cambiar}
        excluidos={excluidos}
        onAlternar={(u) =>
          setExcluidos((previo) => (previo.includes(u) ? previo.filter((x) => x !== u) : [...previo, u]))
        }
        valoresParticipantes={valoresParticipantes}
        onValorParticipante={(u, v) => setValoresParticipantes((previo) => ({ ...previo, [u]: v }))}
        vista={vigenteVista?.resultado ?? null}
        avisoVista={vigenteVista?.aviso ?? null}
        errores={envio.errores}
        cargando={envio.cargando}
        onEnviar={registrar}
        onCancelar={onVolver}
      />
    </div>
  );
}
