'use client';

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface PingResponse {
  mensaje: string;
  version: string;
  latencia_ms: number;
}

export default function PingScreen() {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [data, setData] = useState<PingResponse | null>(null);

  const handleVerificarConexion = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await invoke<PingResponse>('ping_servidor');
      setData(response);
    } catch (err: unknown) {
      if (typeof err === 'string') {
        setError(err);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Error desconocido al intentar conectar con el servidor.');
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="w-full max-w-lg mx-auto p-6 bg-white rounded-lg shadow-md border border-gray-200">
      <h2 className="text-xl font-bold mb-4 text-gray-800">
        Estado del Servidor
      </h2>

      <button
        onClick={handleVerificarConexion}
        disabled={loading}
        className="w-full py-2.5 px-4 bg-indigo-600 hover:bg-indigo-700 text-white font-medium rounded-md transition duration-150 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      >
        {loading ? (
          <>
            <span className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></span>
            Verificando...
          </>
        ) : (
          'Verificar Conexión con el Servidor'
        )}
      </button>

      <div className="mt-6 space-y-4">
        {loading && (
          <div className="p-4 bg-blue-50 border border-blue-200 text-blue-700 rounded-md text-sm">
            Cargando datos de conexión...
          </div>
        )}

        {error && (
          <div className="p-4 bg-red-50 border border-red-200 text-red-700 rounded-md text-sm">
            <p className="font-semibold">Error de Conexión:</p>
            <p>{error}</p>
          </div>
        )}

        {data && !loading && !error && (
          <div className="p-4 bg-green-50 border border-green-200 text-gray-800 rounded-md space-y-2 text-sm">
            <div className="flex items-center justify-between pb-2 border-b border-green-200">
              <span className="font-medium">Estado:</span>
              <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold bg-green-100 text-green-800">
                Conectado
              </span>
            </div>

            <div className="grid grid-cols-2 gap-2 pt-1">
              <div>
                <span className="text-gray-500 block text-xs">Mensaje:</span>
                <span className="font-medium">{data.mensaje}</span>
              </div>
              <div>
                <span className="text-gray-500 block text-xs">Versión:</span>
                <span className="font-medium">{data.version}</span>
              </div>
              <div className="col-span-2">
                <span className="text-gray-500 block text-xs">Latencia:</span>
                <span className="font-medium">{data.latencia_ms} ms</span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}