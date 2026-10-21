#include <stdbool.h>
void probe_clear(void);
bool probe_active(void);
bool probe_show(void *, const char *, const char *, bool, double, double, double, double, double);
void probe_hide(const char *);
char *probe_events(void);
void probe_free(char *);
