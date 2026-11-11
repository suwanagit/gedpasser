import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '@fontsource-variable/nunito'
import './styles/tokens.css'
import './styles/base.css'
import './styles/figure.css'
import './styles/shell.css'
import './styles/screens.css'
import './styles/subjects.css'
import App from './App.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
