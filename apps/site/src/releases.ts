import assets from './download-assets.json';
import selection from './releases.json';

const repository = 'https://github.com/colehank/khor';
const releaseUrl = (version: string) => repository + '/releases/tag/' + version;
const {version, channel, files} = selection;

export const release = {
  version, channel, url: releaseUrl(version),
  downloads: assets.filter(({file}) => files.includes(file)).map(item => ({
    ...item, url: repository + '/releases/download/' + version + '/' + item.file,
  })),
};

export const preview = selection.previewVersion ? {
  version: selection.previewVersion, url: releaseUrl(selection.previewVersion),
} : null;
