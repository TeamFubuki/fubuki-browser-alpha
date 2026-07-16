import { render } from 'solid-js/web';
import InternalPages from './pages/InternalPages';
import './styles/internal-pages.css';

document.body.classList.add('internal-page');
render(() => <InternalPages />, document.getElementById('root') as HTMLElement);
