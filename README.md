# LyricFrame RS

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Vercel](https://img.shields.io/badge/vercel-%23000000.svg?style=for-the-badge&logo=vercel&logoColor=white)
![Spotify](https://img.shields.io/badge/Spotify-1ED760?style=for-the-badge&logo=spotify&logoColor=white)

Un widget en SVG para tu perfil de GitHub que muestra en tiempo real lo que estás escuchando en Spotify. 

Esta es una versión reescrita en Rust, pensada para desplegarse como una función serverless en Vercel. Se apoya en la caché nativa de Vercel para evitar problemas de rate-limit con la API de Spotify.

<div align="center">

[![Spotify](https://lyricframe-rs-5qss.vercel.app/api)](https://lyric-frame.vercel.app/)

</div>

## Características

- Escrito en Rust (usando `vercel_runtime`).
- El fondo del widget se adapta dinámicamente extrayendo la paleta de colores de la carátula del álbum.
- Evita baneos de la API de Spotify usando el header `stale-while-revalidate`.

## Despliegue en Vercel

1. Haz un fork de este repositorio.
2. Obtén tus credenciales desde el [Spotify Developer Dashboard](https://developer.spotify.com/dashboard/):
   - `SPOTIFY_CLIENT_ID`
   - `SPOTIFY_SECRET_ID`
   - `SPOTIFY_REFRESH_TOKEN`
3. Crea un nuevo proyecto en [Vercel](https://vercel.com/new) e importa tu fork.
4. En la configuración, deja los campos *Build Command* e *Install Command* en blanco (Vercel detecta la carpeta `api/` automáticamente).
5. Añade las 3 variables de entorno.
6. Despliega.

## Créditos

Inspirado en el trabajo de **[novatorem/novatorem](https://github.com/novatorem/novatorem)**. El diseño original proviene de su repositorio, esta versión simplemente lo adapta a Rust.

## Licencia

MIT. Revisa el archivo `LICENSE` para más detalles.