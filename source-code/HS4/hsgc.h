#ifndef HSGC_H
#define HSGC_H

#include <stddef.h>

// Opaque handle for GC-managed objects
typedef struct HSGC_Object HSGC_Object;

// Initialize the GC system
void hsgc_init(void);

// Allocate memory with GC
void* hsgc_alloc(size_t size);

// Mark an object as reachable (for roots)
void hsgc_mark(HSGC_Object* obj);

// Collect garbage (mark-and-sweep)
void hsgc_collect(void);

// Finalize the GC system
void hsgc_fini(void);

// Register a root pointer
void hsgc_register_root(void** root);

// Unregister a root
void hsgc_unregister_root(void** root);

#endif // HSGC_H
