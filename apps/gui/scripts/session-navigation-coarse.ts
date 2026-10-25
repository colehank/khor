import {useCoarse} from '@/hooks/use-coarse';
export function useSessionPreviewCoarse(){return useCoarse()||new URLSearchParams(location.search).get('phone')==='1';}
