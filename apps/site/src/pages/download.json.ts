import {release} from '../releases';
export const prerender = true;
export function GET() { return Response.json(release); }
