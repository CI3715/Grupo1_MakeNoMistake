// app/components/AltaCategoria.tsx
import React, { useState } from 'react';

interface CategoriasState {
  Ingreso: string[];
  Gasto: string[];
}

interface AltaCategoriaProps {
  categoriasExistentes: CategoriasState;
  onAgregarCategoria: (tipo: 'Ingreso' | 'Gasto', nombre: string) => void;
  onCancelar: () => void;
}

export const AltaCategoria: React.FC<AltaCategoriaProps> = ({
  categoriasExistentes,
  onAgregarCategoria,
  onCancelar,
}) => {
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<'Ingreso' | 'Gasto'>('Gasto');
  const [error, setError] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const nombreLimpio = nombre.trim();

    if (!nombreLimpio) {
      setError('El nombre de la categoría no puede estar vacío.');
      return;
    }

    // Verificar si ya existe una categoría con el mismo nombre (sin importar mayúsculas/minúsculas)
    const yaExiste = categoriasExistentes[tipo].some(
      (cat) => cat.toLowerCase() === nombreLimpio.toLowerCase()
    );

    if (yaExiste) {
      setError(`La categoría "${nombreLimpio}" ya existe en ${tipo.toLowerCase()}s.`);
      return;
    }

    onAgregarCategoria(tipo, nombreLimpio);
  };

  return (
    <div className="p-6 max-w-md mx-auto bg-white border rounded-lg shadow-md">
      <h2 className="text-xl font-bold mb-4">Nueva Categoría</h2>

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700">Tipo</label>
          <select
            value={tipo}
            onChange={(e) => {
              setTipo(e.target.value as 'Ingreso' | 'Gasto');
              setError('');
            }}
            className="w-full border p-2 rounded mt-1"
          >
            <option value="Gasto">Gasto</option>
            <option value="Ingreso">Ingreso</option>
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700">Nombre de la Categoría</label>
          <input
            type="text"
            value={nombre}
            onChange={(e) => {
              setNombre(e.target.value);
              setError('');
            }}
            className="w-full border p-2 rounded mt-1"
            placeholder="Ej: Mascotas"
          />
          {error && <p className="text-red-500 text-xs mt-1">{error}</p>}
        </div>

        <div className="flex justify-end gap-2 pt-4">
          <button type="button" onClick={onCancelar} className="px-4 py-2 border rounded">
            Cancelar
          </button>
          <button type="submit" className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700">
            Crear Categoría
          </button>
        </div>
      </form>
    </div>
  );
};