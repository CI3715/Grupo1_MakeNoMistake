'use client';

import React from 'react';
import type { ResumenImportacionDto } from '../../types/copia';
import { textoNuevos, textoYaExistian } from '../../utils/copiaFormato';
import { temaSocial } from '../../utils/temaSocial';
import { CopiaEstadisticas } from './CopiaEstadisticas';
import { CopiaListaRechazados } from './CopiaListaRechazados';

/**
 * Resultado de importar: «0 nuevos, 44 ya existían», los tres contadores y, si los hay, los
 * eventos rechazados con su motivo (RF-INT-02, ESC-04).
 */
export function CopiaResumenImportacion({
  modoOscuro,
  resumen,
}: {
  modoOscuro: boolean;
  resumen: ResumenImportacionDto;
}) {
  const tema = temaSocial(modoOscuro);
  return (
    <div className="space-y-3">
      <div role="status" className={`rounded-xl border p-3 text-xs ${tema.avisoOk}`}>
        <p className="font-semibold">Importación terminada.</p>
        <p className="mt-0.5 opacity-90">
          {textoNuevos(resumen.nuevos)}, {textoYaExistian(resumen.ya_existian)}.
          {resumen.nuevos === 0 && resumen.rechazados.length === 0
            ? ' Tu libro ya tenía todo lo que traía esta copia.'
            : ''}
        </p>
      </div>
      <CopiaEstadisticas
        modoOscuro={modoOscuro}
        datos={[
          { etiqueta: 'Nuevos', valor: resumen.nuevos },
          { etiqueta: 'Ya existían', valor: resumen.ya_existian },
          { etiqueta: 'Rechazados', valor: resumen.rechazados.length },
        ]}
      />
      <CopiaListaRechazados modoOscuro={modoOscuro} rechazados={resumen.rechazados} />
    </div>
  );
}
