// Website documents open for reading; the production editor is unchanged.
export * from '../../src/views/FileView';
import {defaultFaceOf as appFace,rendererOf} from '../../src/views/FileView';
export const defaultFaceOf=(path:string):'source'|'preview'=>rendererOf(path)==='md'?'preview':appFace(path);
